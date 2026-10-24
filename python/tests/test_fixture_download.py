"""Exercise fixture retries without network requests or real sleeps."""

from __future__ import annotations

import importlib.util
from email.message import Message
from http.client import IncompleteRead
from io import BytesIO
from pathlib import Path
from unittest.mock import Mock, call
from urllib.error import HTTPError, URLError

import pytest


SPEC = importlib.util.spec_from_file_location(
    "download_fixture", Path(__file__).resolve().parents[2] / "scripts/download_fixture.py"
)
downloader = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(downloader)
URL = "https://example.org/fixture.raw"


def http_error(code=429, retry_after=None):
    headers = Message()
    if retry_after is not None:
        headers["Retry-After"] = retry_after
    return HTTPError(URL, code, "fixture request failed", headers, BytesIO())


@pytest.fixture
def transfer(monkeypatch, tmp_path):
    opener = Mock()
    sleep = Mock()
    monkeypatch.setattr(downloader.urllib.request, "urlopen", opener)
    monkeypatch.setattr(downloader.time, "sleep", sleep)
    return opener, sleep, tmp_path / "vendor/fixture.raw"


@pytest.mark.parametrize(
    "retry_after, delay", [(None, 2), ("7", 7), ("0", 2), ("bad", 2), ("86400", 60)]
)
def test_429_then_success(transfer, retry_after, delay):
    opener, sleep, destination = transfer
    error = http_error(retry_after=retry_after)
    opener.side_effect = [error, BytesIO(b"complete fixture")]

    assert downloader.download_file(URL, destination) == destination

    assert destination.read_bytes() == b"complete fixture"
    assert not destination.with_name("fixture.raw.part").exists()
    assert opener.call_count == 2
    sleep.assert_called_once_with(delay)
    assert error.fp.closed
    request = opener.call_args.args[0]
    assert request.full_url == URL
    assert request.get_header("User-agent") == "OpenMassSpec-CI/1.0"
    assert opener.call_args.kwargs == {"timeout": 180}


def test_http_date_retry_after(transfer, monkeypatch):
    from datetime import datetime, timezone

    opener, sleep, destination = transfer
    clock = Mock()
    clock.now.return_value = datetime(2026, 10, 14, 12, 0, 0, tzinfo=timezone.utc)
    monkeypatch.setattr(downloader, "datetime", clock)
    opener.side_effect = [
        http_error(retry_after="Wed, 14 Oct 2026 12:00:09 GMT"),
        BytesIO(b"fixture"),
    ]

    downloader.download_file(URL, destination)

    sleep.assert_called_once_with(9)
    assert destination.read_bytes() == b"fixture"


def test_exhausted_retries_name_url(transfer):
    opener, sleep, destination = transfer
    opener.side_effect = [http_error() for _ in range(5)]

    with pytest.raises(RuntimeError) as failure:
        downloader.download_file(URL, destination)

    assert URL in str(failure.value)
    assert "after 5 attempt(s)" in str(failure.value)
    assert "429" in str(failure.value)
    assert isinstance(failure.value.__cause__, HTTPError)
    assert opener.call_count == 5
    assert sleep.call_args_list == [call(2), call(4), call(8), call(16)]
    assert not destination.exists()
    assert not destination.with_name("fixture.raw.part").exists()


def test_permanent_http_error_fails_immediately(transfer):
    opener, sleep, destination = transfer
    opener.side_effect = http_error(404)

    with pytest.raises(RuntimeError, match=URL):
        downloader.download_file(URL, destination)

    assert opener.call_count == 1
    sleep.assert_not_called()
    assert not destination.exists()


@pytest.mark.parametrize("error", [http_error(403), http_error(503), URLError("connection reset"), TimeoutError()])
def test_other_transient_failures(transfer, error):
    opener, sleep, destination = transfer
    opener.side_effect = [error, BytesIO(b"fixture")]

    downloader.download_file(URL, destination)

    assert destination.read_bytes() == b"fixture"
    sleep.assert_called_once_with(2)


def test_interrupted_body_restarts_transfer(transfer):
    opener, sleep, destination = transfer
    response = Mock()
    response.__enter__ = Mock(return_value=response)
    response.__exit__ = Mock(return_value=False)
    response.read.side_effect = [b"partial data", IncompleteRead(b"", 10)]
    opener.side_effect = [response, BytesIO(b"good")]

    downloader.download_file(URL, destination)

    assert destination.read_bytes() == b"good"
    assert not destination.with_name("fixture.raw.part").exists()
    sleep.assert_called_once_with(2)


def test_failed_transfer_preserves_existing_fixture(transfer):
    opener, sleep, destination = transfer
    destination.parent.mkdir(parents=True)
    destination.write_bytes(b"previous fixture")
    response = Mock()
    response.__enter__ = Mock(return_value=response)
    response.__exit__ = Mock(return_value=False)
    response.read.side_effect = [b"partial data", IncompleteRead(b"", 10)]
    opener.side_effect = [response] + [http_error() for _ in range(4)]

    with pytest.raises(RuntimeError, match=URL):
        downloader.download_file(URL, destination)

    assert destination.read_bytes() == b"previous fixture"
    assert not destination.with_name("fixture.raw.part").exists()


def test_truncated_body_restarts_transfer(transfer):
    opener, sleep, destination = transfer
    headers = Message()
    headers["Content-Length"] = "1000"
    response = Mock()
    response.__enter__ = Mock(return_value=response)
    response.__exit__ = Mock(return_value=False)
    response.headers = headers
    response.read.side_effect = [b"x" * 100, b""]
    opener.side_effect = [response, BytesIO(b"good")]

    downloader.download_file(URL, destination)

    assert destination.read_bytes() == b"good"
    assert not destination.with_name("fixture.raw.part").exists()
    sleep.assert_called_once_with(2)
