# Mouse Clipboard - Logitech G502 HERO

Configuracion de botones extra del mouse Logitech G502 HERO para manejo del portapapeles y captura de pantalla en Linux.

## Sistema

- **SO**: Manjaro Linux
- **Escritorio**: KDE Plasma 6.5.5
- **Sesion**: Wayland
- **Mouse**: Logitech G502 HERO Gaming Mouse

## Solucion

Se utiliza **libratbag/ratbagctl** para configurar macros de teclado directamente en la memoria onboard del mouse.

### Ventajas

- La configuracion se guarda en el mouse (no necesita software corriendo)
- Funciona despues de reiniciar
- Funciona en cualquier PC donde conectes el mouse
- Compatible con Wayland

## Configuracion Actual

| Boton | Accion | Atajo |
|-------|--------|-------|
| **Atras** (Button 3) | Copiar | `Ctrl+Shift+C` |
| **Adelante** (Button 4) | Portapapeles KDE | `Meta+V` |
| **Click central** (Button 2) | Original | Pegar seleccion, cerrar tabs |
| **Rueda derecha** (Button 9) | Pegar | `Ctrl+V` |
| **Rueda izquierda** (Button 10) | Captura de pantalla | `Meta+Shift+S` |

## Comandos Utiles

### Ver configuracion actual
```bash
sudo systemctl start ratbagd
ratbagctl "singing-gundi" info
```

### Modificar un boton
```bash
# Sintaxis: +KEY_* = presionar, -KEY_* = soltar
ratbagctl "singing-gundi" profile 0 button N action set macro +KEY_X -KEY_X
```

### Ejemplo: Configurar boton para Ctrl+C
```bash
ratbagctl "singing-gundi" profile 0 button 3 action set macro \
  +KEY_LEFTCTRL +KEY_LEFTSHIFT +KEY_C -KEY_C -KEY_LEFTSHIFT -KEY_LEFTCTRL
```

### Restaurar boton a funcion original
```bash
ratbagctl "singing-gundi" profile 0 button 3 action set button 4
```

## Codigos de Teclas Comunes

| Tecla | Codigo |
|-------|--------|
| Ctrl izq | `KEY_LEFTCTRL` |
| Shift izq | `KEY_LEFTSHIFT` |
| Alt izq | `KEY_LEFTALT` |
| Meta/Super | `KEY_LEFTMETA` |
| Insert | `KEY_INSERT` |
| A-Z | `KEY_A` ... `KEY_Z` |
| 0-9 | `KEY_0` ... `KEY_9` |

Ver todos: `/usr/include/linux/input-event-codes.h`

## Notas

- `Ctrl+Shift+C` se usa en lugar de `Ctrl+C` para evitar matar procesos en terminales
- Click central conserva funcion original (pegar seleccion, cerrar pestanas en navegador)
- `Meta+V` es el atajo nativo de KDE para mostrar el historial del portapapeles
- `Meta+Shift+S` abre Spectacle para captura de region de pantalla

## Proyecto Original (Rust)

El directorio contiene un intento inicial en Rust usando evdev/uinput que fue descartado por complejidad. La solucion final usa ratbagctl que es mas simple y robusta.

## Requisitos

```bash
# Ya instalado en Manjaro con Piper
sudo pacman -S libratbag
```
