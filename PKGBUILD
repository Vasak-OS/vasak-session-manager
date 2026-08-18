# Maintainer: Joaquin (Pato) Decima <jdecima@vasak.net.ar>
pkgname=vasak-session-manager
pkgver=0.3.0
pkgrel=1
pkgdesc="VasakOS session manager: display manager / greeter (Rust + Vue + Tauri)"
arch=('x86_64')
url="https://github.com/Vasak-OS/vasak-session-manager"
license=('MIT')
# Runs as a greetd greeter (greetd owns PAM/seat/session), so no pam/systemd
# service is shipped here. cage hosts the greeter; greetd invokes the launcher.
# vasakos-wallpapers holds the image the login screen is drawn on.
# gtk-session-lock es lo que le permite a la pantalla de bloqueo crear las
# superficies del protocolo ext-session-lock; sin eso no hay bloqueo posible.
depends=('webkit2gtk-4.1' 'gtk3' 'cage' 'greetd' 'vasakos-wallpapers' 'gtk-session-lock' 'pam')
makedepends=('git' 'cargo' 'bun' 'rust')
source=("git+${url}.git")
sha256sums=('SKIP')
install="vasak-session-manager.install"

# makepkg's global LTO injects -flto into CFLAGS/LDFLAGS. Crates that build C or
# assembly then emit LTO bitcode that rustc's linker cannot resolve. Rust does
# its own LTO via the Cargo release profile, so makepkg's is redundant here as
# well as harmful.
options=('!lto')

build() {
    cd "$srcdir/$pkgname"
    bun install
    bun run tauri build
}

package() {
    cd "$srcdir/$pkgname"

    install -Dm755 "src-tauri/target/release/$pkgname" \
        "$pkgdir/usr/bin/$pkgname"
    install -Dm755 "packaging/$pkgname-launch" \
        "$pkgdir/usr/bin/$pkgname-launch"

    # Reference configuration, not /etc/greetd/config.toml: that path belongs to
    # the greetd package, and two packages owning one file is a conflict pacman
    # refuses to install. The install scriptlet puts it in place.
    install -Dm644 "packaging/greetd.toml" \
        "$pkgdir/usr/share/$pkgname/greetd.toml"

    # The i18n plugin resolves locales at runtime from a real directory; without
    # this the installed greeter renders raw keys instead of text.
    # La pantalla de bloqueo: el mismo binario del greeter no, pero sí la misma
    # interfaz y el mismo paquete, para que no puedan divergir.
    install -Dm755 "src-tauri/target/release/vasak-lock-screen" \
        "$pkgdir/usr/bin/vasak-lock-screen"

    # Su propio servicio PAM: `login` decide si alguien puede entrar al sistema
    # y rechazaría a quien ya tiene la sesión abierta.
    install -Dm644 "packaging/vasak-lock-screen.pam" \
        "$pkgdir/etc/pam.d/vasak-lock-screen"

    install -dm755 "$pkgdir/usr/share/$pkgname/locales"
    install -Dm644 src-tauri/locales/*.yml \
        "$pkgdir/usr/share/$pkgname/locales/"

    # Creates the state directory the greeter remembers the last account in.
    # It runs as `greeter`, whose home is `/`, so it has nowhere else to write
    # that survives a reboot.
    install -Dm644 "packaging/$pkgname.tmpfiles.conf" \
        "$pkgdir/usr/lib/tmpfiles.d/$pkgname.conf"
}
