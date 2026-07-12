"""Tests for Workstream G5: buffer support, lazy loading, iteration, and filtering."""
import pytest


class TestBinaryBuffer:
    """Tests for BinaryBuffer protocol."""

    def test_create_from_bytes(self):
        from eggsec._core import BinaryBuffer

        buf = BinaryBuffer(b"hello world")
        assert len(buf) == 11

    def test_to_bytes(self):
        from eggsec._core import BinaryBuffer

        buf = BinaryBuffer(b"\x00\x01\x02\x03")
        result = buf.to_bytes()
        assert result == b"\x00\x01\x02\x03"

    def test_hex(self):
        from eggsec._core import BinaryBuffer

        buf = BinaryBuffer(b"\xde\xad\xbe\xef")
        assert buf.hex() == "deadbeef"

    def test_from_hex(self):
        from eggsec._core import BinaryBuffer

        buf = BinaryBuffer.from_hex("cafebabe")
        assert buf.to_bytes() == b"\xca\xfe\xba\xbe"

    def test_from_hex_odd_length(self):
        from eggsec._core import BinaryBuffer

        with pytest.raises(ValueError, match="even length"):
            BinaryBuffer.from_hex("abc")

    def test_from_hex_invalid(self):
        from eggsec._core import BinaryBuffer

        with pytest.raises(ValueError):
            BinaryBuffer.from_hex("xyz")

    def test_memoryview(self):
        from eggsec._core import BinaryBuffer

        buf = BinaryBuffer(b"test data")
        mv = buf.memoryview()
        assert mv is not None
        assert len(mv) == 9

    def test_buffer_protocol(self):
        from eggsec._core import BinaryBuffer

        buf = BinaryBuffer(b"protocol test")
        mv = memoryview(buf)
        assert len(mv) == 13
        assert bytes(mv) == b"protocol test"

    def test_len(self):
        from eggsec._core import BinaryBuffer

        assert len(BinaryBuffer(b"")) == 0
        assert len(BinaryBuffer(b"a")) == 1
        assert len(BinaryBuffer(b"12345")) == 5

    def test_equality(self):
        from eggsec._core import BinaryBuffer

        a = BinaryBuffer(b"same")
        b = BinaryBuffer(b"same")
        c = BinaryBuffer(b"diff")
        assert a == b
        assert a != c

    def test_repr(self):
        from eggsec._core import BinaryBuffer

        buf = BinaryBuffer(b"abc")
        assert "BinaryBuffer" in repr(buf)
        assert "len=3" in repr(buf)


class TestLazyArtifact:
    """Tests for LazyArtifact load/unload."""

    def test_create(self, tmp_path):
        from eggsec._core import ArtifactMeta, LazyArtifact

        p = tmp_path / "test.bin"
        p.write_bytes(b"lazy data")
        meta = ArtifactMeta("test.bin", "binary", "application/octet-stream", 9)
        art = LazyArtifact(str(p), meta)
        assert art.name() == "test.bin"
        assert art.kind() == "binary"
        assert art.mime_type() == "application/octet-stream"
        assert art.size_bytes() == 9
        assert not art.is_loaded()

    def test_load(self, tmp_path):
        from eggsec._core import ArtifactMeta, LazyArtifact

        p = tmp_path / "data.bin"
        p.write_bytes(b"hello lazy")
        meta = ArtifactMeta("data.bin", "binary", "application/octet-stream", 10)
        art = LazyArtifact(str(p), meta)

        buf = art.load()
        assert art.is_loaded()
        assert buf.to_bytes() == b"hello lazy"

    def test_unload(self, tmp_path):
        from eggsec._core import ArtifactMeta, LazyArtifact

        p = tmp_path / "unload.bin"
        p.write_bytes(b"unload me")
        meta = ArtifactMeta("unload.bin", "binary", "application/octet-stream", 9)
        art = LazyArtifact(str(p), meta)
        art.load()
        assert art.is_loaded()

        art.unload()
        assert not art.is_loaded()

    def test_load_caches(self, tmp_path):
        from eggsec._core import ArtifactMeta, LazyArtifact

        p = tmp_path / "cache.bin"
        p.write_bytes(b"cached data")
        meta = ArtifactMeta("cache.bin", "binary", "application/octet-stream", 11)
        art = LazyArtifact(str(p), meta)

        buf1 = art.load()
        buf2 = art.load()
        assert buf1.to_bytes() == buf2.to_bytes()

    def test_path(self, tmp_path):
        from eggsec._core import ArtifactMeta, LazyArtifact

        p = tmp_path / "path.bin"
        p.write_bytes(b"path test")
        meta = ArtifactMeta("path.bin", "binary", "application/octet-stream", 9)
        art = LazyArtifact(str(p), meta)
        assert str(art.path()) == str(p)

    def test_metadata(self, tmp_path):
        from eggsec._core import ArtifactMeta, LazyArtifact

        p = tmp_path / "meta.bin"
        p.write_bytes(b"meta")
        meta = ArtifactMeta("meta.bin", "screenshot", "image/png", 4, content_hash="abc123")
        art = LazyArtifact(str(p), meta)
        assert art.content_hash == "abc123"


class TestPaginatedResults:
    """Tests for PaginatedResults iteration."""

    def test_create(self):
        from eggsec._core import PaginatedResults

        items = [1, 2, 3, 4, 5]
        pr = PaginatedResults(items, page_size=2)
        assert len(pr) == 5
        assert pr.total_pages() == 3

    def test_iteration(self):
        from eggsec._core import PaginatedResults

        items = [10, 20, 30]
        pr = PaginatedResults(items)
        result = list(pr)
        assert result == [10, 20, 30]

    def test_get_page(self):
        from eggsec._core import PaginatedResults

        items = list(range(10))
        pr = PaginatedResults(items, page_size=3)
        page0 = pr.get_page(0)
        assert list(page0) == [0, 1, 2]
        page1 = pr.get_page(1)
        assert list(page1) == [3, 4, 5]
        page3 = pr.get_page(3)
        assert list(page3) == [9]

    def test_get_page_beyond(self):
        from eggsec._core import PaginatedResults

        items = [1, 2]
        pr = PaginatedResults(items, page_size=10)
        page = pr.get_page(5)
        assert list(page) == []

    def test_get_page_info(self):
        from eggsec._core import PaginatedResults

        items = list(range(7))
        pr = PaginatedResults(items, page_size=3)
        info = pr.get_page_info(1)
        assert info["page"] == 1
        assert info["total_pages"] == 3
        assert info["total_items"] == 7
        assert info["has_next"] is True
        assert info["has_prev"] is True
        assert list(info["items"]) == [3, 4, 5]

    def test_to_list(self):
        from eggsec._core import PaginatedResults

        items = [100, 200, 300]
        pr = PaginatedResults(items)
        result = list(pr.to_list())
        assert result == [100, 200, 300]

    def test_reset(self):
        from eggsec._core import PaginatedResults

        items = [1, 2, 3]
        pr = PaginatedResults(items)
        first = list(pr)
        pr.reset()
        second = list(pr)
        assert first == second

    def test_count(self):
        from eggsec._core import PaginatedResults

        items = [1, 2, 3, 4]
        pr = PaginatedResults(items)
        assert pr.count() == 4

    def test_empty(self):
        from eggsec._core import PaginatedResults

        pr = PaginatedResults([])
        assert len(pr) == 0
        assert pr.total_pages() == 0
        assert list(pr) == []

    def test_zero_page_size(self):
        from eggsec._core import PaginatedResults

        items = [1, 2, 3]
        pr = PaginatedResults(items, page_size=0)
        # page_size defaults to 100
        assert pr.total_pages() == 1


class TestArtifactBuffer:
    """Tests for MilestoneArtifact buffer protocol."""

    def test_with_content(self):
        from eggsec._core import MilestoneArtifact

        art = MilestoneArtifact.with_content(
            "id1", "test.bin", "application/octet-stream", b"content data", "hash1"
        )
        assert art.has_content()
        assert art.size_bytes == 12

    def test_to_bytes(self):
        from eggsec._core import MilestoneArtifact

        data = b"binary content"
        art = MilestoneArtifact.with_content("id2", "f.bin", "application/octet-stream", data, "h")
        result = art.to_bytes()
        assert result is not None
        assert bytes(result) == data

    def test_hex(self):
        from eggsec._core import MilestoneArtifact

        art = MilestoneArtifact.with_content("id3", "h.bin", "application/octet-stream", b"\xab\xcd", "h")
        assert art.hex() == "abcd"

    def test_memoryview(self):
        from eggsec._core import MilestoneArtifact

        data = b"memview test"
        art = MilestoneArtifact.with_content("id4", "m.bin", "application/octet-stream", data, "h")
        mv = art.memoryview()
        assert mv is not None
        assert bytes(mv) == data

    def test_buffer_protocol(self):
        from eggsec._core import MilestoneArtifact

        data = b"buffer proto"
        art = MilestoneArtifact.with_content("id5", "b.bin", "application/octet-stream", data, "h")
        mv = memoryview(art)
        assert bytes(mv) == data

    def test_no_content(self):
        from eggsec._core import MilestoneArtifact

        art = MilestoneArtifact("id6", "n.bin", "text/plain", 0, "h")
        assert not art.has_content()
        assert art.to_bytes() is None
        assert art.hex() is None
        assert art.memoryview() is None


class TestFindingSetIteration:
    """Tests for FindingSet iteration and filtering."""

    def _make_finding(self, id, title, severity, category):
        from eggsec._core import Finding

        return Finding(id, title, severity, "target", category, "desc")

    def test_iter(self):
        from eggsec._core import FindingSet, Severity

        fs = FindingSet()
        fs.add_finding(self._make_finding("f1", "F1", Severity.High, "cat1"))
        fs.add_finding(self._make_finding("f2", "F2", Severity.Low, "cat2"))

        items = list(fs)
        assert len(items) == 2

    def test_len(self):
        from eggsec._core import FindingSet, Severity

        fs = FindingSet()
        assert len(fs) == 0
        fs.add_finding(self._make_finding("f1", "F1", Severity.High, "cat1"))
        assert len(fs) == 1

    def test_filter_by_severity(self):
        from eggsec._core import FindingSet, Severity

        fs = FindingSet()
        fs.add_finding(self._make_finding("f1", "F1", Severity.High, "cat1"))
        fs.add_finding(self._make_finding("f2", "F2", Severity.Low, "cat1"))
        fs.add_finding(self._make_finding("f3", "F3", Severity.High, "cat1"))

        filtered = fs.filter_by_severity(Severity.High)
        assert len(filtered) == 2

    def test_filter_by_type(self):
        from eggsec._core import FindingSet, Severity

        fs = FindingSet()
        fs.add_finding(self._make_finding("f1", "F1", Severity.High, "vuln"))
        fs.add_finding(self._make_finding("f2", "F2", Severity.Low, "info"))
        fs.add_finding(self._make_finding("f3", "F3", Severity.Medium, "vuln"))

        filtered = fs.filter_by_type("vuln")
        assert len(filtered) == 2

    def test_to_dicts(self):
        from eggsec._core import FindingSet, Severity

        fs = FindingSet()
        fs.add_finding(self._make_finding("f1", "F1", Severity.High, "cat"))
        dicts = fs.to_dicts()
        assert len(dicts) == 1
        assert dicts[0]["id"] == "f1"

    def test_iter_lazy(self):
        from eggsec._core import FindingSet, Severity

        fs = FindingSet()
        fs.add_finding(self._make_finding("f1", "F1", Severity.High, "cat"))
        fs.add_finding(self._make_finding("f2", "F2", Severity.Low, "cat"))

        it = fs.iter_lazy()
        items = list(it)
        assert len(items) == 2

    def test_contains(self):
        from eggsec._core import FindingSet, Severity

        fs = FindingSet()
        f1 = self._make_finding("f1", "F1", Severity.High, "cat")
        fs.add_finding(f1)
        assert f1 in fs

    def test_repr(self):
        from eggsec._core import FindingSet

        fs = FindingSet()
        assert "FindingSet" in repr(fs)


class TestEventLogLazy:
    """Tests for EventLog lazy iteration."""

    def test_iter_lazy(self):
        from eggsec._core import EventLog, ExecutionEvent

        log = EventLog()
        log.push(ExecutionEvent("h1", "start", 1000))
        log.push(ExecutionEvent("h1", "end", 2000))

        lazy = log.iter_lazy()
        items = list(lazy)
        assert len(items) == 2

    def test_lazy_len(self):
        from eggsec._core import EventLog, ExecutionEvent

        log = EventLog()
        log.push(ExecutionEvent("h1", "event", 1000))
        log.push(ExecutionEvent("h2", "event", 2000))
        log.push(ExecutionEvent("h3", "event", 3000))

        lazy = log.iter_lazy()
        assert len(lazy) == 3
