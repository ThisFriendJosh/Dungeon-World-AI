import gzip
import importlib.util
import io
import json
from pathlib import Path
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("setup_scrying", Path(__file__).resolve().parents[1] / "scripts/setup_scrying.py")
setup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(setup)


class NuGetMetadataTests(unittest.TestCase):
    def test_gzip_and_plain_json_produce_the_same_package_record(self):
        record = {"catalogEntry": {"packageHashAlgorithm": "SHA512", "packageHash": "example"}}
        plain = json.dumps(record).encode("utf-8")
        for payload in (plain, gzip.compress(plain), b"\xef\xbb\xbf" + plain):
            with self.subTest(compressed=payload.startswith(b"\x1f\x8b")):
                with patch.object(setup.urllib.request, "urlopen", return_value=io.BytesIO(payload)):
                    self.assertEqual(setup.fetch_json("metadata-fixture"), record)


if __name__ == "__main__":
    unittest.main()
