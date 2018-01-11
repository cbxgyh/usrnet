use clap;

pub trait App {
    fn with_defaults(self) -> Self;
}

impl<'a, 'b> App for clap::App<'a, 'b> {
    fn with_defaults(self) -> Self {
        self.arg(
            clap::Arg::with_name("tap")
                .long("tap")
                .value_name("TAP")
                .help("Linux TAP interface")
                .default_value("tap0")
                .takes_value(true),
        ).arg(
                clap::Arg::with_name("dev-mac")
                    .long("dev-mac")
                    .value_name("MAC")
                    .help("MAC address of the device")
                    .default_value("00:01:02:03:04:05")
                    .takes_value(true),
            )
            .arg(
                clap::Arg::with_name("dev-ipv4")
                    .long("dev-ipv4")
                    .value_name("IPV4")
                    .help("IPv4 address of the device")
                    .default_value("10.0.0.103")
                    .takes_value(true),
            )
    }
}
