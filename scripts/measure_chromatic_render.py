#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
"""Read physical LVGL platform measurements; never flashes or qualifies a release."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import struct
import time
import zlib

CASES = ('idle', 'counter', 'page', 'full')

def validate_report(value, version, mode, case):
    required = ('elapsed_us', 'handler_calls', 'handler_total_us', 'handler_max_us',
                'flushes', 'transfer_bytes', 'frames', 'frame_total_us', 'frame_max_us',
                'free_heap_bytes', 'largest_free_block_bytes', 'boot_min_before_window_bytes',
                'window_min_free_bytes', 'gui_stack_free_bytes')
    if any(type(value.get(key)) is not int or value[key] < 0 for key in required):
        raise ValueError('Missing or invalid physical counters')
    if not (value.get('schema_version') == 1
            and value.get('execution_mode') == 'physical-lvgl-platform-probe'
            and value.get('firmware_version') == version and value.get('mode') == mode
            and value.get('case') == case and value.get('updates') == 32
            and value.get('core') == 1
            and value.get('cpu_hz_before') == value.get('cpu_hz_after') == 160000000
            and value.get('draw_buffer_bytes') == {'stock-full':46080, 'stripe16':5120, 'stripe48':15360,'stripe48-o2':15360}[mode]
            and value['frames'] == (0 if case == 'idle' else 32)):
        raise ValueError('Wrong physical case, firmware, buffer, core, or frequency')
    if case == 'idle':
        if any(value[key] for key in ('flushes','transfer_bytes','frame_total_us','frame_max_us')):
            raise ValueError('Unexpected rendering during idle case')
    elif (value['flushes'] < 32 or not value['transfer_bytes']
          or value['transfer_bytes'] % 1024 or not value['frame_total_us']):
        raise ValueError('Incomplete physical rendering counters')

def png(rgb565):
    if len(rgb565) != 160 * 144 * 2:
        raise ValueError('Incomplete physical framebuffer')
    rows = bytearray()
    for y in range(144):
        rows.append(0)
        for x in range(160):
            offset = (y * 160 + x) * 2
            value = rgb565[offset] | rgb565[offset + 1] << 8
            red, green, blue = value >> 11, value >> 5 & 63, value & 31
            rows.extend(((red << 3) | red >> 2, (green << 2) | green >> 4, (blue << 3) | blue >> 2))
    def chunk(kind, data):
        return struct.pack('>I',len(data)) + kind + data + struct.pack('>I',zlib.crc32(kind+data)&0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR',struct.pack('>IIBBBBB',160,144,8,2,0,0,0)) + chunk(b'IDAT',zlib.compress(rows)) + chunk(b'IEND',b'')

def receive(connection, marker, timeout):
    received = bytearray()
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        received.extend(connection.read(4096))
        if len(received) > 1024 * 1024:
            raise ValueError('Oversized diagnostic output')
        if marker in received:
            return bytes(received)
    raise TimeoutError('Physical diagnostic response did not finish')

def send(connection, command):
    connection.reset_input_buffer()
    connection.write(command.encode('ascii')+b'\r')
    connection.flush()

def main():
    import serial
    from serial.tools import list_ports
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port',required=True)
    parser.add_argument('--firmware-version',required=True)
    parser.add_argument('--application-sha256',required=True)
    parser.add_argument('--mode',required=True,choices=('stock-full','stripe16','stripe48','stripe48-o2'))
    parser.add_argument('--repeat',type=int,default=3)
    parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--capture',type=Path,required=True)
    parser.add_argument('--resume-capture',action='store_true',help='Resume only the readback after all physical samples were saved')
    args=parser.parse_args()
    if not 1 <= args.repeat <= 10 or not re.fullmatch('[0-9a-f]{64}',args.application_sha256):
        parser.error('Invalid sample count or installed application digest')
    if args.capture.exists() or (args.output.exists() and not args.resume_capture):
        parser.error('Keep existing measurements; choose new output paths')
    samples=[]
    if args.resume_capture:
        saved=json.loads(args.output.read_text())
        samples=saved['samples']
        if (saved.get('complete') is not False or saved.get('application_sha256')!=args.application_sha256
                or len(samples)!=args.repeat*len(CASES)):
            parser.error('Resume requires a matching partial report with every sample')
        for index,value in enumerate(samples):
            validate_report(value,args.firmware_version,args.mode,CASES[index%len(CASES)])
            if not (value.get('firmware_version')==args.firmware_version and value.get('mode')==args.mode
                    and value.get('case')==CASES[index%len(CASES)] and value.get('sample')==index//len(CASES)+1):
                parser.error('Saved physical samples do not match the requested capture')
    matches=[port for port in list_ports.comports() if port.device==args.port and port.vid==0x374e and port.pid==0x0101]
    if len(matches)!=1:
        parser.error('Selected port is not the expected Chromatic interface')
    connection=serial.Serial(port=None,baudrate=115200,timeout=.1)
    connection.dtr=False
    connection.rts=False
    connection.port=args.port
    connection.open()
    try:
        time.sleep(4)
        send(connection,'fwversion')
        data=receive(connection,b'"chromatic"',5)
        data+=connection.read_until(b'\n',size=4096)
        versions=[json.loads(match) for match in re.findall(rb'\{[^\r\n]+\}',data)]
        # The legacy OSD has a 16-byte version buffer including its terminator.
        # This is only a readiness hint. Every measured report below must match
        # the complete app-description version, and installation verifies bytes.
        expected='v'+args.firmware_version
        if not any(value.get('mcu') in (expected,expected[:15]) for value in versions):
            raise ValueError('Physical firmware version mismatch')
        deadline=time.monotonic()+25
        while True:
            send(connection,'wifi_status')
            data=receive(connection,b'CWJSON',5)
            # Marker arrival may precede the complete single-line JSON.
            data+=connection.read_until(b'\n',size=4096)
            states=[json.loads(match) for match in re.findall(rb'\{[^\r\n]+\}',data)]
            if any(value.get('state')=='connected' for value in states):
                break
            if time.monotonic()>=deadline:
                raise ValueError('Wi-Fi not connected for the comparison')
            time.sleep(1)
        print('Device-native Wi-Fi connected; resuming capture.' if args.resume_capture else 'Device-native Wi-Fi connected; beginning physical rendering cases.',flush=True)
        for repeat in range(0 if args.resume_capture else args.repeat):
            for case in CASES:
                send(connection,'render_probe '+case)
                data=receive(connection,b'RENDER_REPORT ',15)
                data+=connection.read_until(b'\n',size=8192)
                matches=re.findall(rb'RENDER_REPORT (\{[^\r\n]+\})',data)
                if len(matches)!=1:
                    raise ValueError('Missing complete physical report')
                value=json.loads(matches[0])
                validate_report(value,args.firmware_version,args.mode,case)
                value['sample']=repeat+1
                samples.append(value)
                # Preserve completed samples immediately if later hardware access fails.
                args.output.parent.mkdir(parents=True,exist_ok=True)
                args.output.write_text(json.dumps({'schema_version':1,'execution_mode':'physical-lvgl-platform-probe',
                    'complete':False,'application_sha256':args.application_sha256,'samples':samples},indent=2)+'\n')
                print(json.dumps({key:value[key] for key in ('sample','case','frames','transfer_bytes','frame_total_us','free_heap_bytes','window_min_free_bytes')}),flush=True)
        # Readback is deliberately outside every measurement window. It needs a
        # temporary full-frame buffer and is not a physical USB-video capture.
        send(connection,'render_probe show')
        receive(connection,b'RENDER_SHOWN',5)
        time.sleep(.2)
        for capture_attempt in range(1,4):
            send(connection,'pocket_screen')
            data=receive(connection,b'POCKETFRAME_END',35)
            rows=[line.strip() for line in data.splitlines() if re.fullmatch(rb'[0-9a-f]{640}',line.strip())]
            if len(rows)==144:
                break
            if capture_attempt==3:
                raise ValueError('Physical render-buffer readback is incomplete')
            print('Interleaved diagnostic output; retrying read-only capture outside measurement.',flush=True)
        raw=bytes.fromhex(b''.join(rows).decode('ascii'))
        args.capture.parent.mkdir(parents=True,exist_ok=True)
        args.capture.write_bytes(png(raw))
        result={'schema_version':1,'execution_mode':'physical-lvgl-platform-probe','complete':True,
            'application_sha256':args.application_sha256,'wifi_connected':True,
            'frame_source':'physical-mcu-render-buffer-over-usb','capture_outside_measurement':True,
            'capture_attempts':capture_attempt,'framebuffer_sha256':hashlib.sha256(raw).hexdigest(),
            'capture_sha256':hashlib.sha256(args.capture.read_bytes()).hexdigest(),'samples':samples}
        args.output.write_text(json.dumps(result,indent=2)+'\n')
        print('Physical measurements and render-buffer readback completed.',flush=True)
    finally:
        connection.close()

if __name__=='__main__':
    main()
