import subprocess
import unittest
from pathlib import Path


INSTALLER = Path(__file__).with_name("install.sh")


class InstallerDisabledTest(unittest.TestCase):
    def test_installer_refuses_unowned_standalone_channel(self) -> None:
        result = subprocess.run(
            ["sh", str(INSTALLER)],
            check=False,
            capture_output=True,
            text=True,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("@ceasarxuu/whalecode@latest", result.stderr)
        self.assertNotIn("openai", result.stderr.lower())


if __name__ == "__main__":
    unittest.main()
