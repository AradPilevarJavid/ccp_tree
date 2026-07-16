pkgname=ccp_tree
pkgver=0.3.2
pkgrel=1
pkgdesc="Generate AI-friendly project trees"
arch=('x86_64' 'aarch64')
url="https://github.com/AradPilevarJavid/ccp_tree"
license=('MIT')
depends=('gcc-libs')
makedepends=('cargo')
source=("https://crates.io/api/v1/crates/$pkgname/$pkgver/download")
sha256sums=('SKIP')

build() {
    cd "$srcdir"

    cargo build --release --locked
}

package() {
    install -Dm755 target/release/ccp_tree \
        "$pkgdir/usr/bin/ccp_tree"
}

