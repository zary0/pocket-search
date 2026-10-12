# SPDX-License-Identifier: GPL-3.0-only
import copy
import importlib.util
import json
from pathlib import Path
import unittest
ROOT=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('source_registry',ROOT/'scripts/generate_chromatic_sources.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
class Sources(unittest.TestCase):
    def setUp(self): self.config=json.loads(module.CONFIG.read_text())
    def test_checked_in_registry_matches_reviewed_config(self):self.assertEqual(module.OUTPUT.read_text(),module.registry(self.config))
    def test_private_and_ambiguous_destinations_fail_before_generation(self):
        for base in ['http://www.w3.org/a/','https://127.0.0.1/a/','https://www.w3.org:443/a/','https://user@www.w3.org/a/','https://unit.local/a/','https://www.w3.org/a/../','https://www.w3.org/a/?q=x']:
            c=copy.deepcopy(self.config);c['profiles'][1]['base']=base
            with self.assertRaises(ValueError):module.registry(c)
    def test_invalid_profile_size_and_seed_scope_fail(self):
        for change in [{'name':'x'*25},{'seed':'https://www.w3.org/Outside/'},{'robots':'https://other.org/robots.txt'},{'scope':'all-web'}]:
            c=copy.deepcopy(self.config);c['profiles'][1].update(change)
            with self.assertRaises(ValueError):module.registry(c)
        c=copy.deepcopy(self.config);c['profiles']*=5
        with self.assertRaises(ValueError):module.registry(c)
    def test_legacy_ip_and_lan_names_fail_with_coherent_seed_and_robots(self):
        for host in ['local-router','127.1','127.0.0.01','0x7f.0x0.0x0.0x1','a'*64+'.org']:
            profile={'name':'Boundary','base':'https://'+host+'/a/','seed':'https://'+host+'/a/node1.html','robots':'https://'+host+'/robots.txt','scope':'node-html'}
            c={'schema_version':1,'profiles':[profile]}
            with self.subTest(host=host),self.assertRaises(ValueError):module.registry(c)
