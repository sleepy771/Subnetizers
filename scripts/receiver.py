#!/usr/bin/env python3
import socket

UDP_IP = '127.0.0.1'
UDP_PORT = 6789


if __name__ == '__main__':
    rec_socket = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

    rec_socket.bind((UDP_IP, UDP_PORT))

    while True:
        data, addr = rec_socket.recvfrom(1024)
        print('Got data from: {}\n'.format(addr))
        print('Data:\n{}'.format(data))
