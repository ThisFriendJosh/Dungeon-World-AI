import importlib.util
import unittest
from pathlib import Path
from PIL import Image, ImageDraw

spec = importlib.util.spec_from_file_location("worker", Path(__file__).with_name("worker.py"))
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)


class CutoutTests(unittest.TestCase):
    def test_white_border_is_removed_and_enclosed_white_is_preserved(self):
        image = Image.new("RGB", (20, 20), "white")
        draw = ImageDraw.Draw(image)
        draw.rectangle((4, 4, 15, 15), fill="black")
        draw.rectangle((7, 7, 12, 12), fill="white")
        result = worker.cutout(image)
        self.assertEqual(result.getpixel((0, 0))[3], 0)
        self.assertEqual(result.getpixel((5, 5))[3], 255)
        self.assertEqual(result.getpixel((8, 8))[3], 255)

    def test_colored_background_is_not_removed(self):
        result = worker.cutout(Image.new("RGB", (8, 8), (20, 100, 20)))
        self.assertEqual(result.getextrema()[3], (255, 255))


if __name__ == "__main__":
    unittest.main()
