# Notas de investigación verificable

Revisión realizada el 2026-08-04 sobre repositorios y archivos primarios de GitHub. No se copió código.

| Proyecto | Arquitectura/uso observado | Licencia verificada | Actividad observada | Riesgo o issue relevante | Decisión |
|---|---|---|---|---|---|
| LocalSend | Flutter; REST/HTTPS; red local y multicast | Apache-2.0 | commit 0aa8038, 2026-08-04 | protocolo discute resume y mDNS (#31, #34) | referencia LAN, no fork |
| LocalSend protocol | REST v2.1; v3 añadido recientemente | no hay LICENSE en raíz | commit bf371ab, 2026-07-27 | ambigüedad/ejemplos (#32) | documentación solamente hasta aclarar licencia |
| RaptorQR | monorepo web/CLI; raptorq+fast_qr WASM; ZXing | MIT | commit fdb434e, 2026-07-13 | repositorio muy reciente; claims requieren benchmark propio | referencia de pipeline |
| cberner/raptorq | Rust, RFC 6330, bindings opcionales | Apache-2.0 | commit 83cf194, 2026-06-22 | ARM32 intrinsics (#216), wheels (#220) | candidato FEC; probar targets móviles |
| Decimen | transferencia óptica/fountain y cámara | MIT | commit 29cba8f, 2026-08-04 | cámara/rotación y layout QR (#23, #21) | referencia práctica |
| TXQR | Go/JS histórico, QR animado | MIT | último commit d92929c, 2019-01-10 | inactivo | referencia histórica |
| libcimbar | C++/WASM, código visual de alta densidad | MPL-2.0 | commit 681e18e, 2026-07-14 | MPL no está en lista inicial aceptada; WebGL (#169) | investigación, no integrar |
| ZXing-C++ | C++ multiplataforma, decodificación | Apache-2.0 | commit 3e09874, 2026-07-31 | CocoaPods rezagado (#925) | candidato nativo, validar packaging |
| fast_qr | Rust/WASM, matrices/render QR | MIT | commit 7bc495e, 2025-06-13 | selección de máscara reportada (#77) | candidato, bloquear hasta evaluar issue/test vectors |
| flutter_rust_bridge | codegen Flutter↔Rust v2 | MIT | commit 1d5348b, 2026-07-11 | issues 2.12 de opaque/web; Qyro es nativo | puente objetivo tras prueba mínima |
| Quinn | QUIC Rust async, rustls | MIT o Apache-2.0 | commit 8663c76, 2026-07-30 | errores UDP/fallback (#2766) | feature flag experimental |

## Hallazgos de diseño

- LocalSend demuestra valor de fallback y puertos configurables, pero Qyro requiere framing/resume/cifrado de contenido propios.
- RaptorQR confirma una ruta técnica Rust RaptorQ + fast_qr + ZXing, sin validar por sí sola Android/iOS/Windows nativos.
- libcimbar no se integra por complejidad y política de licencia inicial.
- QUIC no es MVP; TCP/TLS 1.3 conserva un camino más simple.
- La actividad es una instantánea, no garantía de mantenimiento futuro.

## Fuentes

Enlaces canónicos: [LocalSend](https://github.com/localsend/localsend), [protocolo](https://github.com/localsend/protocol), [RaptorQR](https://github.com/infrost/RaptorQR), [raptorq](https://github.com/cberner/raptorq), [Decimen](https://github.com/bashalarmistalt/decimen-optical-transfer), [TXQR](https://github.com/divan/txqr), [libcimbar](https://github.com/sz3/libcimbar), [ZXing-C++](https://github.com/zxing-cpp/zxing-cpp), [fast_qr](https://github.com/erwanvivien/fast_qr), [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge), [Quinn](https://github.com/quinn-rs/quinn).
