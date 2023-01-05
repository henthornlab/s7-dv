sudo apt install -y build-essential libpcap-dev libnet1-dev libyaml-0-2 libyaml-dev pkg-config zlib1g zlib1g-dev libcap-ng-dev libcap-ng0 make libmagic-dev libgeoip-dev liblua5.1-dev libhiredis-dev libevent-dev python3-yaml rustc cargo libpcre2-dev libjansson-dev

cargo install --force --debug --version 0.14.1 cbindgen

./configure --prefix=/usr/ --sysconfdir=/etc/ --localstatedir=/var --enable-lua