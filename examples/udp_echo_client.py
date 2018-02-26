#!/usr/bin/env python3

import random
import socket
import time


# NOTE: You may need to change these addresses based on your network topology.
BIND_ADDR = ('10.0.0.1',  2048)
ECHO_ADDR = ('10.0.0.103', 4096)


def main():
    '''Sends UDP packets to an echo server and checks the response.'''
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.bind(BIND_ADDR)

    print('Binded to: %s:%d' % BIND_ADDR)

    while True:
        message = [random.randint(0, 255) for _ in range(128)]
        print('Sending %s...' % message)
        sock.sendto(bytes(message), ECHO_ADDR)
        message_, src_addr = '', None

        while src_addr != ECHO_ADDR:
            message_, src_addr = sock.recvfrom(128)

        assert message_ == bytes(message)
        print('Got echo!')

        time.sleep(1)


if __name__ == '__main__':
    main()
