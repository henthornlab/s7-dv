printf(
"Suricata Configuration:\n"
"  AF_PACKET support:                       yes\n"
"  DPDK support:                            no\n"
"  eBPF support:                            no\n"
"  XDP support:                             no\n"
"  PF_RING support:                         no\n"
"  NFQueue support:                         no\n"
"  NFLOG support:                           no\n"
"  IPFW support:                            no\n"
"  Netmap support:                          no \n"
"  DAG enabled:                             no\n"
"  Napatech enabled:                        no\n"
"  WinDivert enabled:                       no\n"
"\n"
"  Unix socket enabled:                     yes\n"
"  Detection enabled:                       yes\n"
"\n"
"  Libmagic support:                        yes\n"
"  libjansson support:                      yes\n"
"  hiredis support:                         no\n"
"  hiredis async with libevent:             no\n"
"  PCRE jit:                                yes\n"
"  LUA support:                             yes\n"
"  libluajit:                               no\n"
"  GeoIP2 support:                          no\n"
"  Non-bundled htp:                         no\n"
"  Hyperscan support:                       no\n"
"  Libnet support:                          yes\n"
"  liblz4 support:                          no\n"
"  Landlock support:                        yes\n"
"\n"
"  Rust support:                            yes\n"
"  Rust strict mode:                        no\n"
"  Rust compiler path:                      /usr/bin/rustc\n"
"  Rust compiler version:                   rustc 1.61.0\n"
"  Cargo path:                              /usr/bin/cargo\n"
"  Cargo version:                           cargo 1.61.0\n"
"\n"
"  Python support:                          yes\n"
"  Python path:                             /usr/bin/python3\n"
"  Install suricatactl:                     yes\n"
"  Install suricatasc:                      yes\n"
"  Install suricata-update:                 yes\n"
"\n"
"  Profiling enabled:                       no\n"
"  Profiling locks enabled:                 no\n"
"\n"
"  Plugin support (experimental):           yes\n"
"\n"
"Development settings:\n"
"  Coccinelle / spatch:                     no\n"
"  Unit tests enabled:                      no\n"
"  Debug output enabled:                    no\n"
"  Debug validation enabled:                no\n"
"  Fuzz targets enabled:                    no\n"
"\n"
"Generic build parameters:\n"
"  Installation prefix:                     /usr\n"
"  Configuration directory:                 /etc/suricata/\n"
"  Log directory:                           /var/log/suricata/\n"
"\n"
"  --prefix                                 /usr\n"
"  --sysconfdir                             /etc\n"
"  --localstatedir                          /var\n"
"  --datarootdir                            /usr/share\n"
"\n"
"  Host:                                    x86_64-pc-linux-gnu\n"
"  Compiler:                                gcc (exec name) / g++ (real)\n"
"  GCC Protect enabled:                     no\n"
"  GCC march native enabled:                yes\n"
"  GCC Profile enabled:                     no\n"
"  Position Independent Executable enabled: no\n"
"  CFLAGS                                   -g -O2 -fPIC -std=c11 -march=native -I${srcdir}/../rust/gen -I${srcdir}/../rust/dist\n"
"  PCAP_CFLAGS                               -I/usr/include\n"
"  SECCFLAGS                                \n"
);
