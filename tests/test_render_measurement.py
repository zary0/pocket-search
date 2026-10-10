# SPDX-License-Identifier: GPL-3.0-only
"""Reject invalid platform evidence without a serial device or pyserial."""
import importlib.util
from pathlib import Path
import unittest
import tempfile

spec = importlib.util.spec_from_file_location('measurement', Path(__file__).parents[1] / 'scripts/measure_chromatic_render.py')
measurement = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measurement)

class PhysicalReportValidation(unittest.TestCase):
    def fixture(self, mode='stock-full', case='counter'):
        return dict(schema_version=1, execution_mode='physical-lvgl-platform-probe',
                    firmware_version='fixture-version', mode=mode, case=case, updates=32, core=1,
                    draw_buffer_bytes={'stock-full':46080,'stripe16':5120,'stripe48':15360,'stripe48-o2':15360}[mode],
                    cpu_hz_before=160000000, cpu_hz_after=160000000, elapsed_us=4000000,
                    handler_calls=100, handler_total_us=600000, handler_max_us=20000,
                    flushes=32, transfer_bytes=46080*32, frames=32, frame_total_us=600000,
                    frame_max_us=20000, free_heap_bytes=60000, largest_free_block_bytes=50000,
                    boot_min_before_window_bytes=40000, window_min_free_bytes=55000,
                    gui_stack_free_bytes=3000)

    def test_changed_identity_or_clock_rejected(self):
        for key, value in [('firmware_version','wrong'), ('execution_mode','emulator'),
                           ('cpu_hz_after',80000000), ('core',0), ('draw_buffer_bytes',0)]:
            report=self.fixture(); report[key]=value
            with self.subTest(key=key), self.assertRaises(ValueError):
                measurement.validate_report(report,'fixture-version','stock-full','counter')

    def test_missing_frames_and_invalid_counters_rejected(self):
        for key, value in [('frames',31), ('transfer_bytes',1023), ('flushes',0),
                           ('window_min_free_bytes',-1), ('frame_total_us',True)]:
            report=self.fixture(); report[key]=value
            with self.subTest(key=key), self.assertRaises(ValueError):
                measurement.validate_report(report,'fixture-version','stock-full','counter')

    def test_idle_requires_no_rendering(self):
        report=self.fixture(case='idle')
        report['frames']=0
        with self.assertRaisesRegex(ValueError,'Unexpected rendering during idle case'):
            measurement.validate_report(report,'fixture-version','stock-full','idle')
        for key in ('frames','flushes','transfer_bytes','frame_total_us','frame_max_us'):
            report[key]=0
        measurement.validate_report(report,'fixture-version','stock-full','idle')

    def test_complete_platform_report_does_not_become_e2e(self):
        for mode in ('stock-full','stripe16','stripe48','stripe48-o2'):
            report=self.fixture(mode=mode)
            measurement.validate_report(report,'fixture-version',mode,'counter')
            self.assertEqual(report['execution_mode'],'physical-lvgl-platform-probe')

    def test_output_aliases_rejected_before_hardware_access(self):
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/'result.json'
            alias=Path(directory)/'nested'/'..'/'result.json'
            with self.assertRaisesRegex(ValueError,'paths must differ'):
                measurement.validate_output_paths(output,alias)
            measurement.validate_output_paths(output,Path(directory)/'frame.png')

    def test_incomplete_frame_rejected(self):
        with self.assertRaises(ValueError):
            measurement.png(b'\x00'*(160*144*2-1))

if __name__=='__main__':
    unittest.main()
