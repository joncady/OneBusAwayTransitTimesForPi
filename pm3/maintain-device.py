#!/usr/bin/env python3
"""Preview or apply device storage retention; defaults to a dry run."""
import subprocess
from retention import main

if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError):
        raise SystemExit('Maintenance stopped; inspect connectivity, mount and configuration. No private values are printed.')
