import socket
import sys

HOST = "127.0.0.1"
PORT = 8765

def send_command(cmd):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.connect((HOST, PORT))
        s.sendall(cmd.encode())

def main():
    if len(sys.argv) < 2:
        print("Usage: igris-player <command> [args]")
        return

    cmd = " ".join(sys.argv[1:])
    send_command(cmd)

if __name__ == "__main__":
    main()
