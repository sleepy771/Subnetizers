#!env python3
import socket
import random

UDP_IP = '127.0.0.1'
UDP_PORT = 6788


def ipv4_gen():
    rnd = random.Random()
    for _ in range(30):
        yield '%d.%d.%d.%d' % (rnd.randint(1, 255),
                               rnd.randint(0, 255),
                               rnd.randint(0, 255),
                               rnd.randint(1, 255))


def make_dgram_msg():
    return ' '.join(ipv4_gen())


if __name__ == '__main__':
    soc = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

    while True:
        soc.sendto(make_dgram_msg(), (UDP_IP, UDP_PORT))
