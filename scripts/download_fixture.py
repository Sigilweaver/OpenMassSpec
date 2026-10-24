"""Download a CI fixture, retrying transient failures without publishing partial files."""

from __future__ import annotations

import argparse
import shutil
import sys
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
from http.client import IncompleteRead
from pathlib import Path


MAX_ATTEMPTS = 5
BACKOFF_SECONDS = 2
MAX_RETRY_AFTER_SECONDS = 60
RETRYABLE_STATUS = {408, 429, 500, 502, 503, 504}


def retry_after_seconds(value: str | None) -> float:
    """Support both Retry-After forms: integer seconds and an HTTP date."""
    if value is None:
        return 0
    value = value.strip()
    if value.isascii() and value.isdigit():
        return float(value)
    try:
        deadline = parsedate_to_datetime(value)
        return max(0, (deadline - datetime.now(timezone.utc)).total_seconds())
    except (ValueError, TypeError, OverflowError):
        return 0


def content_length(response) -> int | None:
    """Return the declared body length, or None when the server sent none."""
    headers = getattr(response, "headers", None)
    value = headers.get("Content-Length") if headers is not None else None
    if isinstance(value, str) and value.strip().isdigit():
        return int(value)
    return None


def download_file(url: str, destination: Path) -> Path:
    """Retry a whole transfer; rename the temporary file only after success."""
    destination.parent.mkdir(parents=True, exist_ok=True)
    partial = destination.with_name(destination.name + ".part")
    request = urllib.request.Request(url, headers={"User-Agent": "OpenMassSpec-CI/1.0"})
    for attempt in range(1, MAX_ATTEMPTS + 1):
        try:
            with urllib.request.urlopen(request, timeout=180) as response, partial.open("wb") as output:
                shutil.copyfileobj(response, output)
                expected = content_length(response)
                if expected is not None and output.tell() != expected:
                    raise IncompleteRead(b"", expected - output.tell())
        except (urllib.error.URLError, TimeoutError, ConnectionError, IncompleteRead) as error:
            delay = BACKOFF_SECONDS * 2 ** (attempt - 1)
            retryable = True
            if isinstance(error, urllib.error.HTTPError):
                retryable = error.code in RETRYABLE_STATUS
                retry_after = error.headers.get("Retry-After") if error.headers else None
                delay = max(delay, min(retry_after_seconds(retry_after), MAX_RETRY_AFTER_SECONDS))
                error.close()
            if not retryable or attempt == MAX_ATTEMPTS:
                raise RuntimeError(
                    f"Failed to download fixture {url} after {attempt} attempt(s): {error}"
                ) from error
            print(
                f"Download failed for {url} (attempt {attempt}/{MAX_ATTEMPTS}): "
                f"{error}; retrying in {delay:g} seconds",
                file=sys.stderr,
                flush=True,
            )
            time.sleep(delay)
        else:
            partial.replace(destination)
            return destination
        finally:
            partial.unlink(missing_ok=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("url")
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    download_file(args.url, args.destination)


if __name__ == "__main__":
    main()
