#!/usr/bin/env python3
import socket
import random
import time

UDP_IP = '127.0.0.1'
UDP_PORT = 6788


def ipv4_gen():
    rnd = random.Random()
    for _ in range(30):
        yield '192.%d.%d.%d' % (rnd.randint(0,255), rnd.randint(0, 255), rnd.randint(1, 255))


def make_dgram_msg():
    return ' '.join(ipv4_gen())


if __name__ == '__main__':
    soc = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

    while True:
        start = time.time()
        for _ in range(10000):
            soc.sendto(bytes(make_dgram_msg(), encoding='ascii'), (UDP_IP, UDP_PORT))
        end = time.time()
        dur = (end - start) * 1000
        print('Sent 300000 ip addresses in %d (ms), what is %d (us) per 1 ip address sent' % (dur, dur * 1000 / 300000))
