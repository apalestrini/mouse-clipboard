use anyhow::{Context, Result};
use evdev::uinput::VirtualDevice;
use evdev::{AttributeSet, Device, EventType, InputEvent, KeyCode, RelativeAxisCode};
use std::env;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// Botones del Logitech G502 que queremos interceptar
const BTN_SIDE: KeyCode = KeyCode::BTN_SIDE; // 275 - Atras -> Copiar
const BTN_EXTRA: KeyCode = KeyCode::BTN_EXTRA; // 276 - Adelante -> Mostrar portapapeles

// Timeout de seguridad: si no hay actividad en 30 segundos, liberar el grab
const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(30);

// Variables de entorno para D-Bus (necesarias cuando se ejecuta con sudo)
static mut DBUS_ADDRESS: Option<String> = None;
static mut DISPLAY: Option<String> = None;
static mut XAUTHORITY: Option<String> = None;
static mut REAL_USER: Option<String> = None;

fn detect_user_environment() {
    // Intentar obtener las variables de entorno del usuario real
    // Primero intentamos las variables directas
    unsafe {
        DBUS_ADDRESS = env::var("DBUS_SESSION_BUS_ADDRESS").ok();
        DISPLAY = env::var("DISPLAY").ok().or(Some(":0".to_string()));
        XAUTHORITY = env::var("XAUTHORITY").ok();
        REAL_USER = env::var("SUDO_USER").ok();

        // Si no tenemos DBUS_ADDRESS, intentamos construirla
        if DBUS_ADDRESS.is_none() {
            if let Ok(sudo_uid) = env::var("SUDO_UID") {
                let bus_path = format!("unix:path=/run/user/{}/bus", sudo_uid);
                DBUS_ADDRESS = Some(bus_path);
            }
        }

        // Si no tenemos XAUTHORITY, intentamos construirla
        if XAUTHORITY.is_none() {
            if let Some(ref user) = REAL_USER {
                XAUTHORITY = Some(format!("/home/{}/.Xauthority", user));
            }
        }

        if let Some(ref addr) = DBUS_ADDRESS {
            println!("[+] DBUS_SESSION_BUS_ADDRESS: {}", addr);
        }
    }
}

fn find_mouse_device() -> Result<Device> {
    let devices = evdev::enumerate();

    for (path, device) in devices {
        let name = device.name().unwrap_or("Unknown");

        // Buscar el mouse Logitech G502
        if name.contains("Logitech") && name.contains("G502") && name.contains("Mouse") {
            if let Some(keys) = device.supported_keys() {
                if keys.contains(BTN_SIDE) && keys.contains(BTN_EXTRA) {
                    println!("[+] Mouse encontrado: {} en {:?}", name, path);
                    return Ok(device);
                }
            }
        }
    }

    anyhow::bail!("No se encontro el mouse Logitech G502")
}

fn create_virtual_mouse(real_device: &Device) -> Result<evdev::uinput::VirtualDevice> {
    let mut builder = VirtualDevice::builder()?.name("Mouse Virtual (mouse-clipboard)");

    // Copiar los ejes relativos del mouse real (movimiento, scroll, etc.)
    if let Some(rel_axes) = real_device.supported_relative_axes() {
        let mut axes = AttributeSet::<RelativeAxisCode>::new();
        for axis in rel_axes.iter() {
            axes.insert(axis);
        }
        builder = builder.with_relative_axes(&axes)?;
    }

    // Copiar las teclas/botones del mouse real, EXCEPTO los que interceptamos
    if let Some(keys) = real_device.supported_keys() {
        let mut key_set = AttributeSet::<KeyCode>::new();
        for key in keys.iter() {
            // NO incluir los botones que queremos interceptar
            if key != BTN_SIDE && key != BTN_EXTRA {
                key_set.insert(key);
            }
        }
        builder = builder.with_keys(&key_set)?;
    }

    let virtual_device = builder.build()?;
    println!("[+] Mouse virtual creado");

    Ok(virtual_device)
}

fn simulate_copy() {
    // Usar xdotool para simular Ctrl+C
    let result = Command::new("xdotool").args(["key", "ctrl+c"]).output();

    match result {
        Ok(output) if output.status.success() => {
            println!("[*] Copiado (Ctrl+C)");
        }
        Ok(output) => {
            eprintln!(
                "[!] xdotool fallo: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => {
            eprintln!("[!] Error ejecutando xdotool: {}", e);
            eprintln!("    Instala xdotool: sudo pacman -S xdotool");
        }
    }
}

fn show_clipboard_panel() {
    // Mostrar el panel de portapapeles de KDE (Klipper)
    let mut cmd = Command::new("qdbus");
    cmd.args(["org.kde.klipper", "/klipper", "showKlipperPopupMenu"]);

    // Pasar las variables de entorno necesarias para D-Bus
    unsafe {
        if let Some(ref addr) = DBUS_ADDRESS {
            cmd.env("DBUS_SESSION_BUS_ADDRESS", addr);
        }
        if let Some(ref display) = DISPLAY {
            cmd.env("DISPLAY", display);
        }
        if let Some(ref xauth) = XAUTHORITY {
            cmd.env("XAUTHORITY", xauth);
        }
    }

    let result = cmd.output();

    match result {
        Ok(output) if output.status.success() => {
            println!("[*] Portapapeles mostrado");
        }
        Ok(output) => {
            eprintln!(
                "[!] qdbus fallo: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => {
            eprintln!("[!] Error ejecutando qdbus: {}", e);
        }
    }
}

fn run() -> Result<()> {
    println!("===========================================");
    println!("       Mouse Clipboard v0.1.0");
    println!("===========================================");
    println!();

    // 0. Detectar variables de entorno del usuario real (importante para sudo)
    detect_user_environment();
    println!();

    println!("Botones configurados:");
    println!("  BTN_SIDE  (Atras)    -> Copiar (Ctrl+C)");
    println!("  BTN_EXTRA (Adelante) -> Mostrar portapapeles");
    println!();
    println!("Seguridad:");
    println!("  - Ctrl+C para salir");
    println!("  - Timeout 30s sin actividad = salir");
    println!();

    // 1. Encontrar el mouse real
    let mut real_mouse = find_mouse_device()?;

    // 2. Crear el mouse virtual ANTES de hacer grab (seguridad)
    let mut virtual_mouse = create_virtual_mouse(&real_mouse)?;

    // 3. Configurar handler de Ctrl+C
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        println!("\n[!] Ctrl+C recibido, liberando mouse...");
        r.store(false, Ordering::SeqCst);
    })?;

    // 4. Ahora si, hacer grab del mouse real
    println!("[+] Haciendo grab del mouse real...");
    real_mouse
        .grab()
        .context("No se pudo hacer grab del mouse. Ejecuta con sudo.")?;
    println!("[+] Grab exitoso - el mouse ahora pasa por nosotros");
    println!();
    println!(">>> Listo! Usa los botones laterales <<<");
    println!();

    let mut last_activity = Instant::now();

    // 5. Loop principal
    while running.load(Ordering::SeqCst) {
        // Verificar timeout de inactividad
        if last_activity.elapsed() > INACTIVITY_TIMEOUT {
            println!("\n[!] Timeout de inactividad (30s), liberando mouse...");
            break;
        }

        // Leer eventos con timeout corto para poder verificar running y timeout
        match real_mouse.fetch_events() {
            Ok(events) => {
                let events: Vec<InputEvent> = events.collect();

                if !events.is_empty() {
                    last_activity = Instant::now();
                }

                for event in events {
                    // Verificar si es un evento de tecla/boton
                    if event.event_type() == EventType::KEY {
                        let key_code = KeyCode::new(event.code());

                        // Solo actuar cuando se PRESIONA (value=1), no al soltar (value=0)
                        if event.value() == 1 {
                            if key_code == BTN_SIDE {
                                simulate_copy();
                                continue; // NO reenviar este evento
                            } else if key_code == BTN_EXTRA {
                                show_clipboard_panel();
                                continue; // NO reenviar este evento
                            }
                        } else if key_code == BTN_SIDE || key_code == BTN_EXTRA {
                            // Tampoco reenviar el release de estos botones
                            continue;
                        }
                    }

                    // Reenviar todos los demas eventos (movimiento, scroll, otros botones)
                    virtual_mouse.emit(&[event])?;
                }
            }
            Err(e) => {
                // EAGAIN/EWOULDBLOCK es normal cuando no hay eventos
                if e.raw_os_error() != Some(11) {
                    eprintln!("[!] Error leyendo eventos: {}", e);
                }
                // Pequena pausa para no consumir CPU
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }

    // 6. Cleanup: liberar el grab
    println!("[+] Liberando grab del mouse...");
    if let Err(e) = real_mouse.ungrab() {
        eprintln!("[!] Error liberando grab: {}", e);
    }
    println!("[+] Mouse liberado correctamente");
    println!("[+] Adios!");

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("\n[ERROR] {}", e);
        eprintln!("\nSugerencias:");
        eprintln!("  - Ejecuta con sudo: sudo ./target/release/mouse-clipboard");
        eprintln!("  - Verifica que el mouse este conectado");
        eprintln!("  - Verifica que /dev/uinput existe");
        std::process::exit(1);
    }
}
