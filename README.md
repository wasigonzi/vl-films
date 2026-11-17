# VL Films

**VL Films** es el editor de video profesional de **Visualab Studios**: línea de tiempo no lineal,
corrección de color, audio, subtítulos y exportación, en el escritorio (Windows, macOS, Linux) y en
el navegador.

- Web: https://www.visualabstudios.com/vl-tools/vl-films
- Visualab Studios: https://www.visualabstudios.com

## Compilar

```sh
cargo run -p filmcraft --release      # app de escritorio
cargo xtask web --serve 8765          # versión web en http://127.0.0.1:8765/
```

Los nombres internos de los paquetes de Rust (`filmcraft-*`), el formato de proyecto y los
comandos de automatización se mantienen por compatibilidad. Documentación técnica: [`docs/`](docs/)
y [`packaging/`](packaging/).

## Licencia

VL Films se basa en software de código abierto:
Copyright (c) 2026 ArtCraft Team and the FilmCraft contributors. Licensed under the MIT License or
the Apache License, Version 2.0, at your option ([LICENSE-MIT](LICENSE-MIT),
[LICENSE-APACHE](LICENSE-APACHE)). Los avisos requeridos están en [NOTICE](NOTICE) y el material
de terceros en [ATTRIBUTION.md](ATTRIBUTION.md).

El nombre y los logotipos de Visualab Studios son marcas de Visualab Studios y no están cubiertos
por la licencia anterior.
