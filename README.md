# Diorama — Proyecto 2: Raytracing (Kevin Villagrán – 23584)

> **Demo (YouTube):**  
> [https://youtu.be](https://www.youtube.com/watch?v=SAUbZULWniY)

Raytracer en CPU con **Raylib** para visualización en tiempo real. Soporta:
- Aceleración por **Uniform Grid** (espacial)
- **Sombras**, **especular Phong**, **reflexión** y **refracción**
- **Texturas** (RGB y RGBA) con estilos (cutout por alpha o luminancia, tintado, “window” coverage)
- **Cubo** y **Slab** (media altura) con texturizado por cara
- **Skybox** cúbico (6 caras)
- Pequeño **builder** tipo voxel con hotbar estilo Minecraft

Repositorio: https://github.com/DerNait/diorama

---

## Índice

1. [Requisitos](#requisitos)
2. [Instalación](#instalación)
3. [Cómo ejecutar](#cómo-ejecutar)
4. [Controles](#controles)
5. [Estructura del proyecto](#estructura-del-proyecto)
6. [Cómo funciona (resumen técnico)](#cómo-funciona-resumen-técnico)
7. [Recursos/Assets](#recursosassets)
8. [Rendimiento y tips](#rendimiento-y-tips)
9. [Solución de problemas](#solución-de-problemas)
10. [Créditos](#créditos)
11. [Licencia](#licencia)

---

## Requisitos

- **Rust** (1.74+ recomendado)

> Las dependencias de Rust (incluida la integración con Raylib vía crate) se compilan automáticamente con `cargo`. No necesitas instalar Raylib nativo por separado.

---

## Instalación

Clona el repo:

```bash
git clone https://github.com/DerNait/diorama
cd diorama
```

Asegúrate de tener la carpeta `assets/` (ver sección [Recursos/Assets](#recursosassets)).

---

## Cómo ejecutar

Modo debug:
```bash
cargo run
```

Modo **optimizado**:
```bash
cargo run --release
```

> Recomendado usar `--release` para escenas con muchos bloques y materiales reflectivos.

---

## Controles

### Cámara (orbital)
- `←` / `→` : orbitar yaw
- `↑` / `↓` : orbitar pitch
- `PageUp` / `PageDown` : zoom (acerca / aleja)

### Luz
- `1` : luz **Puntual**
- `2` : luz **Direccional**

**Direccional (activa con `2`):**
- `J` / `L` : yaw de la luz
- `I` / `K` : pitch de la luz

**Puntual (activa con `1`):**
- `W` / `S` : mover en -Z / +Z
- `A` / `D` : mover en -X / +X
- `R` / `F` : mover en +Y / -Y

### Skybox
- `3` : Skybox 1  
- `4` : Skybox 2

### Builder (bloques)
- `Q` : bloque anterior en hotbar  
- `E` : bloque siguiente en hotbar  
- **Click Izquierdo** : colocar bloque (adyacente a la cara apuntada)  
- **Click Derecho** : quitar bloque apuntado

En pantalla (HUD) verás:
- Hotbar con íconos y selección
- Tips de:
  - “Click izq: colocar”
  - “Click der: quitar”
  - “Light [1: Point, 2: Dir]”
  - “Skybox [3: Sky1, 4: Sky2]”

---

## Estructura del proyecto

- `src/main.rs` — **Punto de entrada**. Configura cámara, luces, carga escena ASCII, skyboxes, HUD y bucle principal.
- `src/accel.rs` — **UniformGridAccel**: grid 3D para acelerar marches de rayos (DDA-like entre celdas).
- `src/camera.rs` — Cámara orbital y base de vectores (eye, forward, right, up).
- `src/light.rs` — Luz **Point** y **Directional** con helpers de orientación/traslación.
- `src/material.rs` — Material (diffuse, albedo[], specular, IOR) + util para convertir a `Color`.
- `src/ray_intersect.rs` — Trait `RayIntersect` y struct `Intersect`.
- `src/cube.rs` — AABB con texturizado por cara y estilos (cutout/tint/window).
- `src/slab.rs` — **Slab** (media altura del bloque), mapeo UV lateral parcial.
- `src/texture.rs` — Carga de PNG a buffer CPU y muestreo (`sample_*`).
- `src/framebuffer.rs` — Framebuffer CPU persistente + **Texture2D** persistente en GPU, **swap sin recreate**.
- `src/scene.rs` — Carga de **capas ASCII** en `assets/scene/*.txt`.
- `src/palette.rs` — Plantillas de cubo por carácter (texturas/estilos por cara).
- `src/skybox.rs` — Muestreo de cubemap (posx/negx/posy/negy/posz/negz).

---

## Cómo funciona (resumen técnico)

1. **Raygen**: por pixel, genera un rayo en espacio mundo usando la base de la cámara y FOV.
2. **Aceleración**: `UniformGridAccel` delimita la escena y reparte objetos en celdas. Un trazado DDA avanza celda a celda y sólo testea AABB de los objetos en la celda actual.
3. **Intersección**:
   - **Cube/Slab**: método de “slabs” (AABB) + determinación de cara impactada y UV.
   - Muestreo **Texture** en CPU (RGB y/o RGBA).  
     Estilos: normal, tintado por luminancia, cutout por luminancia o alpha, y **window** (usa alpha como coverage sin cortar el rayo, útil para vidrio).
4. **Shading**:
   - Difuso “half-lambert” + especular Phong.
   - **Sombras** mediante rayos de oclusión hacia la luz (respetando coverage).
   - **Glints** especulares de alta dureza desde reflejos directos (dependen de tipo de luz).
   - **Reflexión y Refracción** recursivas (profundidad máx. = 3), con **offset de origen** para evitar acne.
5. **Skybox**: muestra color del cubemap cuando no hay hit (o como fondo de reflexión/transmisión).
6. **Framebuffer**: todo el frame se compone en CPU (`Vec<Color>`). Luego, se sube **una** vez por frame a la textura GPU persistente, y se dibuja en la ventana junto con el HUD.

---

## Recursos/Assets

Estructura mínima requerida:

```
assets/
  scene/                 # capas ASCII (*.txt) para la escena
  skyboxes/
    sky1/
      posx.png negx.png posy.png negy.png posz.png negz.png
    sky2/
      posx.png negx.png posy.png negy.png posz.png negz.png

  ui/
    hotbar.png
    hotbar_selection.png

  snow_grass/
    posx.png posy.png negy.png
  dirt/dirt.png
  spruce_log/
    spruce_log.png spruce_log_top.png
  spruce_planks/spruce_planks.png
  glass/glass.png
  spruce_leaves/spruce_leaves.png
  ice/ice.png
  diamond_block/diamond_block.png
  gold_block/gold_block.png
  iron_block/iron_block.png
  lava/lava.png
  diamond_ore/diamond_ore.png
  gold_ore/gold_ore.png
  iron_ore/iron_ore.png
  stone/stone.png
```

> **Escena ASCII:** `assets/scene/*.txt`  
> Cada archivo representa una **capa** en Y. El loader (`scene.rs`) alinea todo a una grilla de tamaño `cube_size` sin gaps.  
> Caracteres mapeados en `src/main.rs` vía `Palette` (ej.: `X` = grass, `D` = dirt, `_`/`-` = slabs, etc.).

---

## Rendimiento y tips

- Ejecuta con `cargo run --release`.
- La aceleración por **grilla uniforme** hace que el coste crezca casi linealmente con los objetos que “tocas” por celda, no con todos los objetos de la escena.
- Materiales como **vidrio/hielo** introducen recursión (reflexión/refracción). Mantener la **profundidad de rebote** en 3 evita explosión de rayos.
- Texturas “window” usan coverage (0..1) sin cortar el rayo principal: da buen look de vidrio sin perder reflejos del fondo.

---

## Solución de problemas

- **Se cierra al iniciar / pantalla negra**  
  Verifica que la carpeta `assets/` exista y que las rutas **coincidan** (sensible a mayúsculas en Linux/macOS).
- **Muy lento**  
  - Corre en `--release`
  - Reduce tamaño de ventana
  - Evita escenas con excesivo vidrio/hielo si tu CPU es limitada
- **Las texturas aparecen volteadas**  
  En skybox ya se corrige el `v`, pero si cambias assets, revisa la convención top-left.

---

## Créditos

- **Autor:** Kevin Villagrán — *Carnet 23584*  
- **Trabajo:** Proyecto 2 — *Raytracing. Diorama*  
- **Librerías:**  
  - [raylib](https://www.raylib.com/) (C)  
  - [raylib-rs](https://github.com/deltaphc/raylib-rs) (Rust bindings)

---

## Licencia

Este proyecto es parte de un curso universitario. Si deseas usar el código, abre un issue en el repo para confirmar la licencia y atribución adecuada.
