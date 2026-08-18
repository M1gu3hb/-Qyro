//! Lo que el ojo tiene que aguantar, probado sin cámara.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "una prueba que no puede fallar en voz alta no es una prueba"
)]

use crate::{Eye, Look, is_above_floor, modules_of_version, pixels_per_module};

// ------------------------------------------------- la aritmética de R10 §8 T1

#[test]
fn las_cifras_de_r10_salen_de_esta_aritmetica_y_no_de_la_memoria() {
    // `R10` §8 T1 da dos números para una v27 con el código al 85 % del alto:
    // **3,07 px/módulo a 640×480 y 4,60 a 1280×720**. Si esta función no los
    // reproduce, o la función está mal o la investigación lo está — y las dos
    // posibilidades importan lo suficiente para que falle una prueba.
    let a_480 = pixels_per_module(27, 480).expect("v27 existe");
    let a_720 = pixels_per_module(27, 720).expect("v27 existe");
    assert!((a_480 - 3.07).abs() < 0.01, "640x480 dio {a_480}");
    assert!((a_720 - 4.60).abs() < 0.01, "1280x720 dio {a_720}");
}

#[test]
fn la_v27_a_640x480_esta_exactamente_en_el_borde() {
    // La trampa entera de `R10` §8 T1 en una línea: el default de CameraX deja
    // la versión que ADR-0044 congeló **por encima del suelo por 0,07**. Pasa,
    // y pasar no es funcionar.
    let ppm = pixels_per_module(27, 480).expect("v27 existe");
    assert!(is_above_floor(27, 480), "quedo por debajo del suelo: {ppm}");
    assert!(
        ppm < 3.2,
        "si esto sube, la aritmetica cambio y la decision de ADR-0048 §3 hay \
         que rehacerla: {ppm}"
    );

    // Y a 720p, que es lo que ADR-0048 §3 pide, está en la banda fiable de 4–5.
    let ppm = pixels_per_module(27, 720).expect("v27 existe");
    assert!((4.0..=5.0).contains(&ppm), "720p dio {ppm}");
}

#[test]
fn bajar_de_version_es_la_palanca_y_se_ve_cuanto_da() {
    // ADR-0048 §3 deja escrito el plan B. Esto comprueba que el plan B **hace
    // algo**: a 640×480 una v22 sube a 3,6 y una v20 a 3,9, que es margen de
    // verdad donde la v27 no lo tiene.
    let v27 = pixels_per_module(27, 480).expect("existe");
    let v22 = pixels_per_module(22, 480).expect("existe");
    let v20 = pixels_per_module(20, 480).expect("existe");
    assert!(v22 > v27 && v20 > v22, "{v27} {v22} {v20}");
    assert!(v20 > 3.8, "la palanca no daba margen: {v20}");
}

#[test]
fn una_version_que_no_existe_no_devuelve_un_numero_creible() {
    // Lo peor que puede hacer un cálculo geométrico es dar un `f64` con pinta
    // de bueno para una entrada sin sentido.
    assert_eq!(modules_of_version(0), None);
    assert_eq!(modules_of_version(41), None);
    assert_eq!(pixels_per_module(41, 1080), None);
    assert!(!is_above_floor(41, 1080));
    // Y el control: los extremos válidos sí responden.
    assert_eq!(modules_of_version(1), Some(21));
    assert_eq!(modules_of_version(40), Some(177));
}

// ------------------------------------------------------------------- el ojo

#[test]
fn un_frame_sin_codigo_no_es_un_error() {
    // El caso más común de todos: la cámara mira la pantalla mientras cambia.
    let mut eye = Eye::new();
    let gris = vec![128_u8; 320 * 240];
    assert_eq!(eye.look(&gris, 320, 240), Look::Nothing);
    assert_eq!(eye.tally(), (1, 0));
    assert_eq!(eye.finish(), None);
}

#[test]
fn un_buffer_mal_medido_se_ignora_en_vez_de_tirar_la_transferencia() {
    // Viene de otro proceso por JNI. Un frame mal medido es un frame malo.
    let mut eye = Eye::new();
    assert_eq!(eye.look(&[0_u8; 10], 320, 240), Look::Nothing);
    assert_eq!(eye.look(&[], 0, 0), Look::Nothing);
    assert_eq!(eye.tally(), (2, 0), "los frames malos tambien se cuentan");
}

#[test]
fn las_dos_cifras_del_recuento_distinguen_lo_que_una_sola_no() {
    // «300 mirados, 2 leidos» y «300 mirados, 280 leidos» son la misma barra de
    // progreso y dos situaciones opuestas: enfoca mal, o va bien.
    let mut eye = Eye::new();
    for _ in 0..5 {
        eye.look(&vec![200_u8; 64 * 64], 64, 64);
    }
    assert_eq!(eye.tally(), (5, 0));
}
