# Maintainer: Vasak Group <maintainer@vasakos.org>
pkgname=vdm
pkgver=0.1.0
pkgrel=1
pkgdesc="VasakOS Display Manager (Rust + Vue + Tauri)"
arch=('x86_64')
url="https://github.com/vasakos/vdm"
license=('MIT')
depends=('webkit2gtk' 'gtk3' 'cage' 'pam' 'nix')
makedepends=('cargo' 'bun' 'tauri')
source=("." ) # Helper to imply local source for this example. Usually git url or tarball.
sha256sums=('SKIP')

build() {
    cd "$srcdir"
    # Identify directory if source is copied (makepkg copies everything or we use git)
    # Assuming we are running this where the source is.
    
    bun install
    bun run tauri build
}

package() {
    cd "$srcdir"
    
    # Binary
    install -Dm755 "src-tauri/target/release/vapp" "$pkgdir/usr/bin/vdm"
    
    # Launcher
    install -Dm755 "packaging/vdm-launch" "$pkgdir/usr/bin/vdm-launch"
    
    # Service
    install -Dm644 "packaging/vdm.service" "$pkgdir/usr/lib/systemd/system/vdm.service"
    
    # Configs (Optional, if any)
}
