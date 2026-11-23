<div align="center">
  <img src="assets/icon.ico" width="64" height="64" alt="Mi Agenda Logo" />
  <h1>Mi Agenda</h1>
  <p><strong>Aplicación de escritorio en Rust para planificación personal, calendario mensual, agenda diaria y vista semanal.</strong></p>

  ![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
  ![Platform](https://img.shields.io/badge/Platform-Windows-0078D6?logo=windows)
  ![Version](https://img.shields.io/badge/Version-1.0.0-4338CA)
</div>

---

## Características principales

- **Tres vistas de planificación**:
  - **Calendario Mensual**: cuadrícula de 7 columnas con navegación por mes, filtros por categoría, indicador de día actual (`HOY`), botón rápido `+` por celda, doble clic para abrir la agenda del día y panel lateral de detalle (`DETALLE DEL DÍA` y `Agregar Rápido`).
  - **Agenda Diaria**: cronograma ordenado por hora con banda lateral de color, línea dinámica de hora actual (`ACTUAL`), progreso de tareas, tarjeta de bienestar (`Ritmo del día`) y notas rápidas con guardado automático.
  - **Vista Semanal**: tablero de 7 columnas (`LUN`–`DOM`) con tarjetas por día, distribución por categorías, bloque editable `Intención de la Semana`, balance de carga y exportación de resumen (`Semana_N_Resumen.txt` en el Escritorio).
- **Tareas flexibles y colores**:
  - Duración de **1 día**, **3 días**, **1 semana** (`7 días`) o personalizada (`1–14 días`).
  - Categorías (`Trabajo`, `Personal`, `Estudio`, `Salud`), prioridades (`Baja`, `Media`, `Alta`) y selector de color RGB personalizado.
- **Modo Claro y Modo Oscuro accesibles**: tema claro editorial (`#F8F9FC` / `#FFFFFF` / índigo `#4338CA`) y tema oscuro de alto contraste, además de **Modo Enfoque** (`En foco`) y búsqueda global (`Ctrl+K` / `Ctrl+F`, limpiar con `Esc`).
- **Integración con Windows**:
  - **Icono en la bandeja del sistema** (`tray-icon`) con insignia numérica de tareas pendientes hoy y resumen al pasar el cursor.
  - **Mini-Calendario flotante** (`Mini`) y **píldora flotante** (`224×34 px`) arrastrable junto a la barra de tareas.
  - **Inicio automático** con Windows mediante `--autostart` e instalador **Inno Setup** (`Mi_Agenda_Setup.exe`).
- **Persistencia local protegida**: los datos se guardan automáticamente en `%LOCALAPPDATA%\organizador_tareas\tareas.dat` en formato binario (`OTSK`), sin exponer archivos JSON en el Escritorio.
- **Auto-actualizador integrado**: comprueba nuevas versiones en [GitHub Releases](https://github.com/AlejandroEspinosaLeal/agenda/releases), descarga `Mi_Agenda_Setup.exe` y actualiza de forma silenciosa (`/SILENT`) conservando intactos tus datos en `tareas.dat`.

> [!NOTE]
> El actualizador solo reemplaza el binario en `%LOCALAPPDATA%\Programs\Mi Agenda\Mi_Agenda.exe`. Tus tareas, notas diarias e intenciones semanales permanecen en `%LOCALAPPDATA%\organizador_tareas\tareas.dat`.

---

## Instalación (Windows)

1. Descarga **`Mi_Agenda_Setup.exe`** desde la última versión en [Releases](https://github.com/AlejandroEspinosaLeal/agenda/releases).
2. Ejecuta el instalador (no requiere permisos de administrador).
3. Opcionalmente deja marcada la casilla **Iniciar automáticamente con Windows** para tener el icono en la bandeja y la píldora de tareas siempre disponibles.

---

## Compilación desde el código fuente

### Requisitos

- [Rust](https://www.rust-lang.org/) (`cargo` y `rustc`, edición 2021)
- *(Opcional)* [UPX](https://upx.github.io/) para comprimir el ejecutable final
- *(Opcional)* [Inno Setup 6](https://jrsoftware.org/isinfo.php) para generar el instalador `Mi_Agenda_Setup.exe`

### Comandos

```powershell
# Ejecutar tests unitarios
cargo test

# Verificar calidad y lints
cargo clippy --all-targets -- -D warnings

# Compilar binario optimizado por tamaño (LTO + opt-level = "z" + strip)
cargo build --release
```

El ejecutable optimizado se genera en `target\release\organizador_tareas.exe`. Para comprimirlo y empaquetar el instalador:

```powershell
# Comprimir con UPX (opcional)
upx --best --lzma target\release\organizador_tareas.exe

# Generar instalador en target\installer\Mi_Agenda_Setup.exe
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
│   ├── app.rs            # Estado principal, barra superior, sidebar, modal y píldora flotante
│   ├── calendar.rs       # Lógica de calendario civil, semanas ISO y aritmética de fechas
│   ├── model.rs          # Modelo Task/TaskStore y persistencia binaria OTSK
│   ├── theme.rs          # Paletas Clarity Suite (claro) y oscuro accesible + fuente Segoe UI
│   ├── tray.rs           # Icono de bandeja de Windows, insignia numérica y menú contextual
│   ├── updater.rs        # Comprobación y descarga de actualizaciones desde GitHub Releases
│   └── views/
│       ├── mod.rs        # Helpers visuales compartidos y filtros de categoría
│       ├── monthly.rs    # Vista Calendario Mensual
│       ├── daily.rs      # Vista Agenda Diaria
│       ├── weekly.rs     # Vista Semanal
│       └── mini.rs       # Vista Mini-Calendario flotante
├── build.rs              # Incrustación de icon.ico en Windows mediante winresource
├── Cargo.toml            # Dependencias (eframe, serde, serde_json, tray-icon) y perfil release
└── installer.iss         # Script de Inno Setup 6 para Mi_Agenda_Setup.exe
```

---

## Atajos de teclado

| Atajo | Acción |
| --- | --- |
| `Ctrl + K` / `Ctrl + F` | Enfocar el buscador global |
| `Esc` | Cerrar modal o limpiar búsqueda |
| `←` / `→` | Mes, día o semana anterior / siguiente (según la vista activa) |
| `Enter` | Guardar tarea desde el campo de título |
