Especificación Técnica: Custom Linux Display Manager (Tauri 2 + Rust + Vue)
1. Visión General
El objetivo es construir un Display Manager (DM) moderno para Linux utilizando Tauri 2. El sistema debe manejar la autenticación de usuarios mediante PAM, gestionar sesiones de usuario de forma segura (dropping privileges) y proporcionar una interfaz fluida construida con Vue.js 3 y Tailwind CSS.

2. Stack Tecnológico
Backend: Rust (Tauri 2).

Frontend: Vue.js 3 (Composition API) + Tailwind CSS + Vite. + bun

Comunicación: Tauri IPC (Commands & Events).

Seguridad/Auth: PAM (Pluggable Authentication Modules).

Manejo de Procesos: nix crate (para syscalls de Unix) y tokio.

Entorno Gráfico: Wayland (vía compositor cage o sway en modo kiosk).

3. Arquitectura de Seguridad (Crucial)
Para evitar los problemas de seguridad de LightDM:

Proceso Maestro (Root): Se encarga de abrir el TTY, inicializar el servidor gráfico y manejar el "switch" de usuario.

Proceso Greeter (Tauri): Se debe ejecutar preferentemente bajo un usuario dedicado de bajos privilegios (ej. greeter-user).

Salto de Privilegios: Tras la validación exitosa de PAM, el proceso maestro debe realizar un fork, usar setuid y setgid para el usuario autenticado, y ejecutar (exec) el entorno de escritorio seleccionado.

4. Requerimientos del Backend (Rust)
La IA debe implementar los siguientes módulos:

Auth Module: Integración con pam-auth. Función authenticate(username, password) que devuelva un Token o booleano.

Session Module: * Detección de sesiones instaladas (leer /usr/share/xsessions y /usr/share/wayland-sessions).

Lógica de spawn_session: Configurar variables de entorno (HOME, PATH, XDG_RUNTIME_DIR) y lanzar el proceso del escritorio.

System Module: Apagado, reinicio y suspensión del sistema.

5. Requerimientos del Frontend (Vue + Tailwind)
La interfaz debe ser minimalista y funcional:

Componentes:

UserSelector.vue: Lista visual de usuarios del sistema.

LoginInput.vue: Campo de contraseña con feedback de error.

SessionSelector.vue: Dropdown para elegir entre GNOME, KDE, Sway, etc (los que esten disponibles en el sistema).

PowerMenu.vue: Botones de Power, Restart y Suspend.

Estado: Usar un Store (Pinia o reactivo simple) para manejar el usuario seleccionado y la sesión elegida.

6. Configuración de Entorno y Despliegue
El proyecto debe incluir un archivo de unidad de systemd (vdm.service).

Instrucciones para correr el DM sobre un TTY específico (normalmente TTY7).

Script de arranque que lance un compositor Wayland mínimo (ej. cage) ejecutando la app de Tauri a pantalla completa.