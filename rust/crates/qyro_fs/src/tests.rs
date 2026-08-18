//! Real directories, real files, real symlinks.
//!
//! Every test here uses a scratch directory under the system temp dir and
//! removes it on drop. Nothing is mocked: a symlink test that does not create a
//! symlink proves nothing about symlinks.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot fail loudly is not a test"
)]

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use qyro_manifest::TransferManifest;
use qyro_transfer::{ContentSink, ContentSource as _};

use crate::error::FsError;
use crate::io::{FileSink, FileSource, HASH_BUFFER_LEN, digest_of, open_part};
use crate::manifest_builder::{PlannedFile, manifest_from_disk};
use crate::resume::ResumeState;
use crate::safe_path;

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A scratch directory, removed on drop.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("qyro-fs-{tag}-{}-{unique}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// Writes `len` deterministic bytes to `path` without holding them all.
fn write_pattern(path: &Path, len: u64) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut file = fs::File::create(path).unwrap();
    let mut written = 0u64;
    let mut buffer = vec![0u8; 8192];
    while written < len {
        let want = ((len - written).min(8192)) as usize;
        for (index, slot) in buffer[..want].iter_mut().enumerate() {
            *slot = ((written + index as u64) % 251) as u8;
        }
        file.write_all(&buffer[..want]).unwrap();
        written += want as u64;
    }
    file.sync_all().unwrap();
}

fn plan(source: &Path, relative: &str) -> PlannedFile {
    PlannedFile {
        source: source.to_path_buf(),
        relative: relative.to_owned(),
    }
}

/// Moves every item of `manifest` from `source_dir` to `sink`, chunk by chunk,
/// exactly the way the engine would.
fn materialise(
    manifest: &TransferManifest,
    source: &FileSource,
    sink: &mut FileSink,
) -> Result<(), FsError> {
    for item in manifest.items() {
        let mut offset = 0u64;
        let mut buffer = vec![0u8; HASH_BUFFER_LEN];
        while offset < item.size() {
            let want = ((item.size() - offset).min(HASH_BUFFER_LEN as u64)) as usize;
            let filled = source.read_at(item.item_id(), offset, &mut buffer[..want]);
            if filled == 0 {
                break;
            }
            sink.put(item.item_id(), offset, &buffer[..filled])?;
            offset += filled as u64;
        }
        sink.finish_item(item.item_id())?;
    }
    Ok(())
}

// ------------------------------------------------------------- the happy path

#[test]
fn a_multi_megabyte_file_arrives_byte_identical() {
    let from = Scratch::new("from");
    let to = Scratch::new("to");
    let size = 5 * 1024 * 1024 + 777;
    write_pattern(&from.path("big.bin"), size);
    write_pattern(&from.path("nested/small.bin"), 1234);

    let files = vec![
        plan(&from.path("big.bin"), "big.bin"),
        plan(&from.path("nested/small.bin"), "nested/small.bin"),
    ];
    let manifest = manifest_from_disk(9, 0, &files).expect("manifest");

    let mut paths = std::collections::BTreeMap::new();
    for (index, file) in files.iter().enumerate() {
        paths.insert((index + 1) as u32, file.source.clone());
    }
    let source = FileSource::new(paths);
    let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");

    materialise(&manifest, &source, &mut sink).expect("transfer");

    // Byte for byte, not by verdict.
    for (original, arrived) in [
        (from.path("big.bin"), to.path("big.bin")),
        (from.path("nested/small.bin"), to.path("nested/small.bin")),
    ] {
        let sent = fs::read(&original).unwrap();
        let got = fs::read(&arrived).unwrap();
        assert_eq!(
            sent.len(),
            got.len(),
            "{} arrived a different length",
            arrived.display()
        );
        assert!(sent == got, "{} differs byte for byte", arrived.display());
    }

    // And no part file was left behind.
    assert!(
        !to.path("big.bin.qyro-part").exists(),
        "the part file survived a successful transfer"
    );
}

#[test]
fn building_a_manifest_from_disk_does_not_load_the_file() {
    use crate::manifest_builder::PEAK_BUILDER_READ;
    let from = Scratch::new("hashmem");
    let small_size = 1024u64;
    let large_size = 2 * HASH_BUFFER_LEN as u64 + 17;
    write_pattern(&from.path("small.bin"), small_size);
    write_pattern(&from.path("large.bin"), large_size);

    let measure = |source: &Path, relative: &str| {
        PEAK_BUILDER_READ.with(|peak| peak.set(0));
        let files = vec![plan(source, relative)];
        let manifest = manifest_from_disk(1, 0, &files).expect("manifest");
        (PEAK_BUILDER_READ.with(std::cell::Cell::get), manifest)
    };
    let (small_peak, small_manifest) = measure(&from.path("small.bin"), "small.bin");
    let (large_peak, large_manifest) = measure(&from.path("large.bin"), "large.bin");

    assert_eq!(
        small_peak, small_size as usize,
        "the small file recorded a read that did not happen"
    );
    assert_eq!(
        large_peak, HASH_BUFFER_LEN,
        "the large file did not use the bounded hash buffer"
    );
    assert!(
        small_peak < large_peak,
        "the counter is a constant rather than the largest completed read"
    );

    // Both builds still produced the right answer, so zero reads or an early
    // return cannot satisfy the measurement.
    assert_eq!(small_manifest.items()[0].size(), small_size);
    assert_eq!(large_manifest.items()[0].size(), large_size);
    assert_eq!(
        large_manifest.items()[0].hash().digest(),
        digest_of(&from.path("large.bin")).unwrap().as_slice()
    );
}

#[test]
fn file_source_peak_is_the_largest_completed_read_not_the_request() {
    let from = Scratch::new("sourcemem");
    let small_size = 1024usize;
    let large_size = 2 * HASH_BUFFER_LEN;
    write_pattern(&from.path("small.bin"), small_size as u64);
    write_pattern(&from.path("large.bin"), large_size as u64);

    let measure = |item_id: u32, path: PathBuf| {
        let mut paths = std::collections::BTreeMap::new();
        paths.insert(item_id, path);
        let source = FileSource::new(paths);
        let mut output = vec![0u8; HASH_BUFFER_LEN];
        let filled = source.read_at(item_id, 0, &mut output);
        (filled, source.peak_read.get())
    };
    let (small_read, small_peak) = measure(1, from.path("small.bin"));
    let (large_read, large_peak) = measure(2, from.path("large.bin"));

    assert_eq!(small_read, small_size);
    assert_eq!(small_peak, small_read);
    assert_eq!(large_read, HASH_BUFFER_LEN);
    assert_eq!(large_peak, large_read);
    assert!(
        small_peak < large_peak,
        "the source counter recorded the request or a constant, not the read"
    );
}

#[test]
fn file_sink_peak_is_the_largest_successful_write_not_a_constant() {
    let measure = |tag: &str, size: usize| {
        let from = Scratch::new(&format!("sinkmemfrom-{tag}"));
        let to = Scratch::new(&format!("sinkmemto-{tag}"));
        write_pattern(&from.path("a.bin"), size as u64);
        let files = vec![plan(&from.path("a.bin"), "a.bin")];
        let manifest = manifest_from_disk(1, 0, &files).expect("manifest");
        let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");

        let refused = vec![0u8; HASH_BUFFER_LEN];
        assert_eq!(
            sink.put(99, 0, &refused).unwrap_err(),
            FsError::DigestMismatch { item_id: 99 }
        );
        assert_eq!(
            sink.peak_write, 0,
            "a refused write was counted as accepted"
        );

        let bytes = fs::read(from.path("a.bin")).unwrap();
        sink.put(1, 0, &bytes).expect("write");
        sink.finish_item(1).expect("finish");
        sink.peak_write
    };

    let small_peak = measure("small", 1024);
    let large_peak = measure("large", HASH_BUFFER_LEN);
    assert_eq!(small_peak, 1024);
    assert_eq!(large_peak, HASH_BUFFER_LEN);
    assert!(
        small_peak < large_peak,
        "the sink counter is a constant rather than the largest accepted write"
    );
}

#[test]
fn the_content_sink_trait_really_writes_the_bytes_it_is_given() {
    let from = Scratch::new("trait-write-from");
    let to = Scratch::new("trait-write-to");
    let bytes = b"written through ContentSink::write_at";
    fs::write(from.path("a.bin"), bytes).unwrap();
    let manifest =
        manifest_from_disk(1, 0, &[plan(&from.path("a.bin"), "a.bin")]).expect("manifest");
    let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");

    ContentSink::write_at(&mut sink, 1, 0, bytes);
    sink.finish_item(1).expect("the trait write must verify");

    assert_eq!(fs::read(to.path("a.bin")).unwrap(), bytes);
}

#[test]
fn opening_without_append_does_not_truncate_before_the_caller_decides() {
    let root = Scratch::new("open-preserves");
    let path = root.path("part.qyro-part");
    let original = b"resume bytes already committed";
    fs::write(&path, original).unwrap();
    let canonical_root = fs::canonicalize(&root.dir).unwrap();

    let handle = open_part(&canonical_root, &path, false).expect("plain part opens");
    assert_eq!(handle.metadata().unwrap().len(), original.len() as u64);
    drop(handle);
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn containment_distinguishes_a_real_child_from_a_real_outsider() {
    let root = Scratch::new("inside-root");
    let outside = Scratch::new("inside-outside");
    fs::create_dir(root.path("child")).unwrap();

    assert!(safe_path::is_inside(&root.dir, &root.path("child")).unwrap());
    assert!(!safe_path::is_inside(&root.dir, &outside.dir).unwrap());
}

#[test]
fn resolving_through_an_existing_directory_is_the_normal_case() {
    let root = Scratch::new("existing-directory");
    fs::create_dir(root.path("already")).unwrap();

    let resolved = safe_path::resolve_under(&root.dir, "already/file.bin")
        .expect("AlreadyExists for a directory is not a refusal");
    assert_eq!(
        resolved.final_path.parent(),
        Some(fs::canonicalize(root.path("already")).unwrap().as_path())
    );
    assert_eq!(
        resolved
            .final_path
            .file_name()
            .and_then(std::ffi::OsStr::to_str),
        Some("file.bin")
    );
}

// ---------------------------------------------------------------- refusals

#[test]
fn a_digest_mismatch_never_produces_the_final_file() {
    let from = Scratch::new("mismatchfrom");
    let to = Scratch::new("mismatchto");
    write_pattern(&from.path("a.bin"), 4096);

    let files = vec![plan(&from.path("a.bin"), "a.bin")];
    let manifest = manifest_from_disk(1, 0, &files).expect("manifest");

    // Change the file's *content* after the manifest was built, keeping its
    // length. Writing a different length would not do: `write_pattern` is a
    // function of the offset, so a longer file has the same first 4096 bytes
    // and the digest would still match — the first version of this test did
    // exactly that and passed for the wrong reason.
    fs::write(from.path("a.bin"), vec![0xAAu8; 4096]).unwrap();

    let mut paths = std::collections::BTreeMap::new();
    paths.insert(1u32, from.path("a.bin"));
    let source = FileSource::new(paths);
    let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");

    let outcome = materialise(&manifest, &source, &mut sink);
    assert_eq!(
        outcome.unwrap_err(),
        FsError::DigestMismatch { item_id: 1 },
        "a changed file was accepted"
    );
    assert!(
        !to.path("a.bin").exists(),
        "the final file appeared despite a digest mismatch"
    );
    assert!(
        !to.path("a.bin.qyro-part").exists(),
        "the part file survived a mismatch; ADR-0027 says nothing verifiable does"
    );
}

#[test]
#[cfg(unix)]
fn a_symlink_in_the_destination_cannot_redirect_a_write() {
    let from = Scratch::new("linkfrom");
    let to = Scratch::new("linkto");
    let elsewhere = Scratch::new("elsewhere");
    write_pattern(&from.path("photo.jpg"), 2048);

    // A real symlink: `photos` in the destination points outside the root.
    std::os::unix::fs::symlink(&elsewhere.dir, to.path("photos")).unwrap();
    assert!(
        fs::symlink_metadata(to.path("photos"))
            .unwrap()
            .file_type()
            .is_symlink(),
        "the fixture failed to create a symlink, so this test proves nothing"
    );

    let files = vec![plan(&from.path("photo.jpg"), "photos/photo.jpg")];
    let manifest = manifest_from_disk(1, 0, &files).expect("manifest");
    let mut paths = std::collections::BTreeMap::new();
    paths.insert(1u32, from.path("photo.jpg"));
    let source = FileSource::new(paths);
    let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");

    let outcome = materialise(&manifest, &source, &mut sink);
    assert!(
        matches!(outcome, Err(FsError::SymlinkInPath { .. })),
        "a symlinked directory did not refuse the write: {outcome:?}"
    );
    assert!(
        fs::read_dir(&elsewhere.dir).unwrap().next().is_none(),
        "content was written through the link, outside the destination root"
    );
}

#[test]
fn an_opened_part_outside_the_root_is_rejected_before_it_can_be_changed() {
    let root = Scratch::new("post-open-root");
    let outside = Scratch::new("post-open-outside");
    let part = outside.path("a.bin.qyro-part");
    let original = b"receiver-owned bytes outside the destination";
    fs::write(&part, original).unwrap();

    let canonical_root = fs::canonicalize(&root.dir).unwrap();
    let outcome = open_part(&canonical_root, &part, false);
    assert!(
        matches!(outcome, Err(FsError::EscapesRoot { .. })),
        "an opened part outside the root was not rejected: {outcome:?}"
    );
    assert_eq!(
        fs::read(part).unwrap(),
        original,
        "the post-open containment check changed an outside file"
    );
}

#[cfg(unix)]
fn symlink_file(target: &Path, link: &Path) -> bool {
    std::os::unix::fs::symlink(target, link).unwrap();
    true
}

/// Crea el enlace, o dice **por qué no pudo** en vez de fallar como si el
/// código estuviera roto.
///
/// Windows exige `SeCreateSymbolicLinkPrivilege` para esto: sin modo
/// desarrollador ni elevación, `symlink_file` devuelve el error 1314 y el
/// `unwrap()` original convertía «a esta consola le falta un privilegio» en un
/// fallo de prueba idéntico a «el resolvedor deja pasar un enlace». Son dos
/// cosas distintas y el registro tiene que distinguirlas: la puerta local se
/// vuelve inservible si un rojo puede significar cualquiera de las dos.
///
/// Los runners de `windows-latest` sí tienen el privilegio, así que **en CI esto
/// se ejecuta de verdad** — que es donde la cobertura tiene que existir. Aquí
/// devuelve `false` y la prueba lo dice en voz alta, porque **saltada no es
/// pasada**.
#[cfg(all(windows, feature = "windows-reparse-test"))]
fn symlink_file(target: &Path, link: &Path) -> bool {
    match std::os::windows::fs::symlink_file(target, link) {
        Ok(()) => true,
        Err(error) if error.raw_os_error() == Some(1314) => {
            println!(
                "SALTADA: esta consola no tiene SeCreateSymbolicLinkPrivilege                  (error 1314). La prueba NO se ha ejecutado. En CI, sobre                  windows-latest, sí corre."
            );
            false
        }
        Err(error) => panic!("no se pudo crear el enlace: {error}"),
    }
}

#[test]
#[cfg(any(unix, all(windows, feature = "windows-reparse-test")))]
fn a_symlink_at_the_final_part_component_is_refused_without_touching_its_target() {
    // This is the path `FileSink` really opens. A resolver-only assertion does
    // not exercise O_NOFOLLOW/FILE_FLAG_OPEN_REPARSE_POINT.
    let from = Scratch::new("finallinkfrom");
    let to = Scratch::new("finallinkto");
    let elsewhere = Scratch::new("finaltarget");
    write_pattern(&from.path("a.bin"), 2048);

    let victim = elsewhere.path("victim.txt");
    let original = b"receiver-owned bytes outside the destination";
    fs::write(&victim, original).unwrap();
    let part_path = to.path("a.bin.qyro-part");
    if !symlink_file(&victim, &part_path) {
        // Sin privilegio no hay enlace que refutar, y afirmar sobre un archivo
        // normal probaría otra cosa mientras aparenta probar ésta.
        return;
    }
    assert!(
        fs::symlink_metadata(&part_path)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the fixture did not put a real link at the part-file component"
    );

    let files = vec![plan(&from.path("a.bin"), "a.bin")];
    let manifest = manifest_from_disk(1, 0, &files).expect("manifest");
    let mut paths = std::collections::BTreeMap::new();
    paths.insert(1u32, from.path("a.bin"));
    let source = FileSource::new(paths);
    let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");

    let outcome = materialise(&manifest, &source, &mut sink);
    assert!(
        matches!(outcome, Err(FsError::SymlinkInPath { .. })),
        "the real FileSink path returned the wrong typed error: {outcome:?}"
    );
    assert_eq!(
        fs::read(&victim).unwrap(),
        original,
        "FileSink wrote through the final-component link"
    );
    assert!(
        !to.path("a.bin").exists(),
        "a refused transfer still produced the final file"
    );
}

#[test]
#[cfg(windows)]
fn a_junction_at_the_final_component_is_classified_as_a_reparse_point() {
    use std::os::windows::fs::MetadataExt as _;

    // Directory junctions are NTFS reparse points and do not require the
    // CreateSymbolicLink privilege. This gives the default Windows suite a real
    // negative fixture even when the file-symlink matrix feature is unavailable.
    let root = Scratch::new("junction-root");
    let outside = Scratch::new("junction-target");
    let junction = root.path("part.qyro-part");
    let status = std::process::Command::new("cmd.exe")
        .args(["/d", "/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&outside.dir)
        .status()
        .expect("cmd.exe must create the junction fixture");
    assert!(status.success(), "the junction fixture was not created");
    assert_ne!(
        fs::symlink_metadata(&junction).unwrap().file_attributes() & 0x0000_0400,
        0,
        "the fixture is not a reparse point"
    );

    let canonical_root = fs::canonicalize(&root.dir).unwrap();
    let outcome = open_part(&canonical_root, &junction, false);
    assert!(
        matches!(outcome, Err(FsError::SymlinkInPath { .. })),
        "the junction was not classified as a final-component link: {outcome:?}"
    );

    // Remove the link itself while both target directories still exist. On
    // Windows RemoveDirectory removes a junction without traversing its target.
    fs::remove_dir(&junction).unwrap();
}

#[test]
fn a_path_that_escapes_the_root_is_refused_at_materialisation() {
    let to = Scratch::new("escape");
    // A string the manifest would refuse, handed straight to the resolver: this
    // is the layer that has to hold even when the one above did not.
    for attempt in ["../outside.bin", "a/../../outside.bin", "./x"] {
        let outcome = safe_path::resolve_under(&to.dir, attempt);
        assert!(
            matches!(outcome, Err(FsError::EscapesRoot { .. })),
            "{attempt} was not refused: {outcome:?}"
        );
    }
    // And a legitimate path still resolves, so the refusals are not "everything
    // fails".
    let good = safe_path::resolve_under(&to.dir, "nested/ok.bin").expect("a normal path resolves");
    assert!(
        good.final_path
            .starts_with(fs::canonicalize(&to.dir).unwrap())
    );
    assert!(safe_path::has_no_traversal(Path::new("a/b/c")));
    assert!(!safe_path::has_no_traversal(Path::new("a/../b")));
}

#[test]
fn an_existing_file_at_the_destination_is_handled_by_policy() {
    let from = Scratch::new("collidefrom");
    let to = Scratch::new("collideto");
    write_pattern(&from.path("a.bin"), 1024);
    fs::write(to.path("a.bin"), b"the receiver's own file").unwrap();

    let files = vec![plan(&from.path("a.bin"), "a.bin")];
    let manifest = manifest_from_disk(1, 0, &files).expect("manifest");
    let mut paths = std::collections::BTreeMap::new();
    paths.insert(1u32, from.path("a.bin"));
    let source = FileSource::new(paths);
    let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");

    let outcome = materialise(&manifest, &source, &mut sink);
    assert!(
        matches!(outcome, Err(FsError::DestinationExists { .. })),
        "an existing file was not refused: {outcome:?}"
    );
    assert_eq!(
        fs::read(to.path("a.bin")).unwrap(),
        b"the receiver's own file",
        "the receiver's file was overwritten, which is the one thing ADR-0027 §2 forbids"
    );
}

// ------------------------------------------------------------------- resume

#[test]
fn resume_metadata_round_trips() {
    let state = ResumeState {
        transfer_id: 0x0102_0304_0506_0708,
        items: vec![
            crate::resume::ItemProgress {
                item_id: 1,
                bytes_committed: 65_536,
            },
            crate::resume::ItemProgress {
                item_id: 2,
                bytes_committed: 0,
            },
        ],
    };
    let encoded = state.encode();
    assert_eq!(ResumeState::decode(&encoded).unwrap(), state);
    assert_eq!(state.progress_of(1), Some(65_536));
    assert_eq!(state.progress_of(9), None);
}

#[test]
fn resume_metadata_from_a_future_version_is_refused_by_version() {
    let state = ResumeState {
        transfer_id: 1,
        items: Vec::new(),
    };
    let mut encoded = state.encode();
    encoded[8] = 2;
    assert_eq!(
        ResumeState::decode(&encoded).unwrap_err(),
        FsError::UnsupportedResumeVersion { found: 2 },
        "a future version was interpreted rather than refused by name"
    );

    // The other refusals of the read order, each by its own route.
    let mut wrong_magic = state.encode();
    wrong_magic[0] ^= 0xFF;
    assert_eq!(
        ResumeState::decode(&wrong_magic).unwrap_err(),
        FsError::NotResumeMetadata
    );

    let mut reserved = state.encode();
    reserved[9] = 1;
    assert_eq!(
        ResumeState::decode(&reserved).unwrap_err(),
        FsError::ResumeReservedNotZero
    );

    assert!(matches!(
        ResumeState::decode(&[0u8; 3]).unwrap_err(),
        FsError::ResumeTruncated { found: 3 }
    ));

    // And the untouched bytes still decode, so none of the above passes because
    // everything fails.
    assert!(ResumeState::decode(&state.encode()).is_ok());
}

#[test]
fn an_interrupted_transfer_resumes_from_its_metadata() {
    let from = Scratch::new("resumefrom");
    let to = Scratch::new("resumeto");
    let size = 3 * HASH_BUFFER_LEN as u64 + 99;
    write_pattern(&from.path("a.bin"), size);

    let files = vec![plan(&from.path("a.bin"), "a.bin")];
    let manifest = manifest_from_disk(42, 0, &files).expect("manifest");
    let mut paths = std::collections::BTreeMap::new();
    paths.insert(1u32, from.path("a.bin"));
    let source = FileSource::new(paths);

    // First run: write one buffer's worth, record progress, then drop the sink
    // — which is what a dead process leaves behind.
    {
        let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");
        let mut buffer = vec![0u8; HASH_BUFFER_LEN];
        let filled = source.read_at(1, 0, &mut buffer);
        sink.put(1, 0, &buffer[..filled]).expect("write");
        sink.persist_progress().expect("metadata");
    }

    let committed = HASH_BUFFER_LEN as u64;
    assert!(
        to.path("a.bin.qyro-part").exists(),
        "the part file did not survive the interruption"
    );
    assert!(
        FileSink::resume_path(&to.dir).exists(),
        "the resume metadata did not survive the interruption"
    );

    // Bytes after the committed boundary model a write that reached the file
    // but not the metadata before the process died. The fresh production sink,
    // not this test, must read `.qyro-resume` and truncate them.
    let mut interrupted = fs::OpenOptions::new()
        .append(true)
        .open(to.path("a.bin.qyro-part"))
        .unwrap();
    interrupted.write_all(&vec![0xA5; size as usize]).unwrap();
    interrupted.sync_all().unwrap();
    drop(interrupted);
    assert!(fs::metadata(to.path("a.bin.qyro-part")).unwrap().len() > size);

    // Second run: the harness knows only the boundary it wrote in the fixture;
    // it never decodes the metadata. `put` must make production apply it.
    let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");
    let mut offset = committed;
    let mut buffer = vec![0u8; HASH_BUFFER_LEN];
    let mut first_resumed_write = true;
    while offset < size {
        let want = ((size - offset).min(HASH_BUFFER_LEN as u64)) as usize;
        let filled = source.read_at(1, offset, &mut buffer[..want]);
        sink.put(1, offset, &buffer[..filled]).expect("write");
        offset += filled as u64;
        if first_resumed_write {
            assert_eq!(
                fs::metadata(to.path("a.bin.qyro-part")).unwrap().len(),
                offset,
                "production did not truncate bytes beyond bytes_committed"
            );
            first_resumed_write = false;
        }
    }
    sink.finish_item(1).expect("the resumed transfer verifies");

    assert_eq!(
        fs::read(to.path("a.bin")).unwrap(),
        fs::read(from.path("a.bin")).unwrap(),
        "the resumed file is not the file that was sent"
    );
}

#[test]
fn a_leftover_part_file_is_recovered_or_discarded_by_policy() {
    let from = Scratch::new("leftoverfrom");
    write_pattern(&from.path("a.bin"), 2048);

    let files = vec![plan(&from.path("a.bin"), "a.bin")];
    let manifest = manifest_from_disk(1, 0, &files).expect("manifest");
    let mut paths = std::collections::BTreeMap::new();
    paths.insert(1u32, from.path("a.bin"));
    let source = FileSource::new(paths);

    for (tag, orphan_len) in [("short", 17usize), ("long", 8192usize)] {
        let to = Scratch::new(&format!("leftover-{tag}"));

        // Orphans on both sides of the real 2048-byte payload. A one-byte
        // accepted write exposes whether production discarded and recreated
        // the part instead of merely overwriting enough of it by accident.
        fs::write(to.path("a.bin.qyro-part"), vec![0xA5; orphan_len]).unwrap();
        assert!(!FileSink::resume_path(&to.dir).exists());

        let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");
        let mut first = [0u8; 1];
        assert_eq!(source.read_at(1, 0, &mut first), 1);
        sink.put(1, 0, &first).expect("first write");
        assert_eq!(
            fs::metadata(to.path("a.bin.qyro-part")).unwrap().len(),
            1,
            "the {tag} orphan was reused instead of discarded"
        );

        materialise(&manifest, &source, &mut sink).expect("transfer");
        assert_eq!(
            fs::read(to.path("a.bin")).unwrap(),
            fs::read(from.path("a.bin")).unwrap(),
            "the {tag} orphan contaminated the result"
        );
    }
}

#[test]
fn resume_metadata_for_another_transfer_makes_the_part_an_orphan() {
    let from = Scratch::new("foreign-resume-from");
    let to = Scratch::new("foreign-resume-to");
    write_pattern(&from.path("a.bin"), 4096);

    let files = vec![plan(&from.path("a.bin"), "a.bin")];
    let manifest = manifest_from_disk(42, 0, &files).expect("manifest");
    fs::write(to.path("a.bin.qyro-part"), vec![0xA5; 8192]).unwrap();
    let foreign = ResumeState {
        transfer_id: 99,
        items: vec![crate::resume::ItemProgress {
            item_id: 1,
            bytes_committed: 4096,
        }],
    };
    fs::write(FileSink::resume_path(&to.dir), foreign.encode()).unwrap();

    let mut paths = std::collections::BTreeMap::new();
    paths.insert(1u32, from.path("a.bin"));
    let source = FileSource::new(paths);
    let mut sink = FileSink::new(&to.dir, &manifest).expect("sink");
    let mut first = [0u8; 1];
    assert_eq!(source.read_at(1, 0, &mut first), 1);
    sink.put(1, 0, &first).expect("first write");
    assert_eq!(
        fs::metadata(to.path("a.bin.qyro-part")).unwrap().len(),
        1,
        "metadata for transfer 99 was trusted by transfer 42"
    );

    materialise(&manifest, &source, &mut sink).expect("transfer");
    assert_eq!(
        fs::read(to.path("a.bin")).unwrap(),
        fs::read(from.path("a.bin")).unwrap()
    );
}
