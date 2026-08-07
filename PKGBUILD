# Maintainer: Joaquin (Pato) Decima <jdecima@vasak.net.ar>
pkgname=vasak-session-manager
pkgver=0.1.0
pkgrel=1
pkgdesc="VasakOS session manager: display manager / greeter (Rust + Vue + Tauri)"
arch=('x86_64')
url="https://github.com/Vasak-OS/vasak-session-manager"
license=('MIT')
# NOTE: the Rust `nix` crate is a cargo dependency, NOT a pacman package
# (pacman's `nix` is the Nix package manager) — it must not be in depends.
depends=('webkit2gtk-4.1' 'gtk3' 'cage' 'pam')
makedepends=('git' 'cargo' 'bun' 'rust')
source=("git+${url}.git")
sha256sums=('SKIP')

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
    install -Dm644 "packaging/$pkgname.service" \
        "$pkgdir/usr/lib/systemd/system/$pkgname.service"
    install -Dm644 "packaging/$pkgname.pam" \
        "$pkgdir/etc/pam.d/$pkgname"
}
