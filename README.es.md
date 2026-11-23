<div align="center">
  <img src="assets/icon.ico" width="64" height="64" alt="Mi Agenda Logo" />
  <h1>Mi Agenda</h1>
  <p><strong>Aplicación de escritorio en Rust para planificación personal, time-blocking, modo enfoque y vista semanal.</strong></p>

  ![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
  ![Platform](https://img.shields.io/badge/Platform-Windows-0078D6?logo=windows)
  ![Version](https://img.shields.io/badge/Version-1.2.0-4338CA)

  <p><a href="README.md">English</a> | <strong>Español</strong></p>
</div>

---

## Características principales

- **Tres vistas interactivas de planificación**:
  - **Calendario Mensual**: cuadrícula de 7 columnas con badge de `HOY`, botón `+` al pasar el ratón, hasta 3 bloques por celda + `+N más`, **selección por arrastre multi-día** para crear tareas de varios días de una vez, e inspector lateral colapsable con notas rápidas autoguardadas.
  - **Agenda Diaria**: rejilla horaria real (`06:00–23:00` o `24h`) con franja de `Todo el día`, línea en vivo de hora actual (`ACTUAL`), algoritmo en Rust puro para mostrar bloques solapados en columnas paralelas y resumen de ritmo diario.
  - **Vista Semanal**: modo dual **Horario** (rejilla compartida de 7 días) y **Tablero**, pie analítico colapsable (horas por categoría, `Intención de la Semana`, balance de carga) y menú de exportación.
- **Interacción directa y Backlog**:
  - Haz clic o arrastra sobre franjas vacías (ajuste automático a 15 min) para crear bloques, arrástralos entre horas o días, estira su duración desde el borde inferior y arrastra tareas de la bandeja **Sin Programar (Backlog)** directamente al calendario.
- **Paleta de Comandos (`Ctrl+K`) y Entrada Natural**:
  - Modal centrado para buscar tareas/eventos/notas, ejecutar comandos y crear bloques escribiendo natural (ej. `"Estudiar Rust 18:00-19:30 #Estudio"` o `"Reunión mañana 10:00 1h #Trabajo"`).
- **Modo Enfoque (`F`)**:
  - Oculta barras laterales, centra el bloque activo y muestra un temporizador configurable **Pomodoro (`25/5`) / Deep Work (`50/10`) / Restante del bloque** junto a sus notas (consumo 0 de CPU en pausa).
- **Exportación real**:
  - Exporta tu semana a **Markdown (`.md`)**, texto plano (`.txt`) o **iCalendar estándar (`.ics` RFC 5545)** con `VCALENDAR`/`VEVENT`/`RRULE`.
- **Paletas de colores y Modo Claro/Oscuro**:
  - 6 paletas accesibles (**Índigo**, **Océano**, **Bosque**, **Terracota**, **Rosa**, **Lavanda**) con preferencia guardada de modo claro u oscuro.
- **Guía interactiva de bienvenida (`?`)**:
  - Tour con spotlight en el primer uso que recorre automáticamente Mensual, Semanal, Diaria, Backlog, `Ctrl+K`, Modo Enfoque y Paletas/Notificaciones (repetible en cualquier momento desde el botón `?`).
- **Integración con Windows y Auto-actualizador**:
  - **Notificación diaria nativa en Windows** una vez al día al iniciar o cambiar de día con tus pendientes (también disponible desde la bandeja o `Ctrl+K`).
  - **Icono en la bandeja del sistema** (`tray-icon`) con insignia numérica, `--autostart` e instalador **Inno Setup** (`Mi_Agenda_Setup.exe`).
  - **Actualización automática desde GitHub Releases**: al detectar una versión nueva en `releases/latest`, descarga `Mi_Agenda_Setup.exe`, instala en silencio (`/SILENT /CLOSEAPPLICATIONS`) y reabre la app manteniendo intactos tus datos en `%LOCALAPPDATA%\organizador_tareas\tareas.dat` (`OTSK` v2 con migración automática desde v1).

---

## Instalación (Windows)

1. Descarga **`Mi_Agenda_Setup.exe`** desde la última versión en [Releases](https://github.com/AlejandroEspinosaLeal/agenda/releases).
2. Ejecuta el instalador (no requiere permisos de administrador).
3. Opcionalmente deja marcada la casilla **Iniciar automáticamente con Windows** para recibir la notificación diaria y tener el icono en la bandeja al encender el PC.

---

## Compilación desde el código fuente

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

Compresión opcional y empaquetado del instalador:

```powershell
upx --best --lzma target\release\organizador_tareas.exe
"C:\Program Files (x86)\Inno Setup 6\ISCC.exe" installer.iss
```

---

## Estructura del proyecto

```text
organizador_tareas/
├── assets/
│   └── icon.ico          # Icono multi-resolución de la aplicación (16..256 px)
├── src/
│   ├── main.rs           # Punto de entrada y soporte de flag --autostart
│   ├── app.rs            # Estado principal, cabecera, sidebar, Ctrl+K y Modo Enfoque
│   ├── calendar.rs       # Lógica de calendario civil, semanas ISO y aritmética de fechas
│   ├── export.rs         # Exportadores Markdown (.md) e iCalendar RFC 5545 (.ics)
│   ├── focus.rs          # Máquina de estados del temporizador Pomodoro / Deep Work
│   ├── model.rs          # Modelo v2 (chrono) y persistencia binaria OTSK con migración
│   ├── nlp.rs            # Parser de entrada rápida en lenguaje natural para Ctrl+K
│   ├── theme.rs          # Paletas accesible claro/oscuro (WCAG AA), espaciados y radios
│   ├── timeblock.rs      # Algoritmo de solapamientos en columnas y ajuste a 15 min
│   ├── tour.rs           # Guía interactiva multi-vista con spotlight
│   ├── tray.rs           # Icono de bandeja de Windows y notificaciones Toast diarias
│   ├── updater.rs        # Descarga e instalación automática desde GitHub Releases
│   └── views/
│       ├── mod.rs        # Helpers visuales compartidos y filtros de categoría
│       ├── monthly.rs    # Vista Calendario Mensual con arrastre multi-día e inspector
│       ├── daily.rs      # Vista Agenda Diaria con rejilla horaria
│       └── weekly.rs     # Vista Semanal dual (Horario / Tablero)
├── build.rs              # Incrustación de icon.ico en Windows mediante winresource
├── Cargo.toml            # Dependencias (chrono, eframe, serde, serde_json, tray-icon)
└── installer.iss         # Script de Inno Setup 6 para Mi_Agenda_Setup.exe
```

---

## Atajos de teclado

| Atajo | Acción |
| --- | --- |
| `Ctrl + K` | Abrir paleta de comandos y entrada rápida en lenguaje natural |
| `Ctrl + F` | Enfocar el filtro de búsqueda de la vista |
| `M` / `W` / `D` | Cambiar a vista Mensual / Semanal / Diaria |
| `T` | Ir a Hoy |
| `C` | Abrir modal de Nueva Tarea / Evento |
| `F` | Activar / Salir de Modo Enfoque |
| `←` / `→` | Mes, semana o día anterior / siguiente |
| `Esc` | Cerrar paleta, modal, guía, Modo Enfoque o inspector |
