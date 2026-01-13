"""Tests for bloom filter module.

Following test-driven-execution: Write tests first (RED), then implement (GREEN).

Bloom filters provide probabilistic set membership with O(1) lookups.
- False positives possible (says "yes" when item wasn't added)
- False negatives impossible (never says "no" for added items)

Used for: "Did chain X touch file Y?" - fast filtering before expensive DB queries.
"""

import pytest


class TestBloomFilterBasic:
    """Test basic bloom filter operations."""

    def test_bloom_filter_creates_with_defaults(self):
        """BloomFilter should create with default parameters.

        RED: Run before implementation - should fail
        GREEN: Implement BloomFilter class
        """
        from context_os_events.index.bloom import BloomFilter

        bloom = BloomFilter()

        assert bloom is not None
        assert bloom.size > 0
        assert bloom.hash_count > 0

    def test_bloom_filter_creates_with_custom_params(self):
        """BloomFilter should accept custom expected_items and false_positive_rate."""
        from context_os_events.index.bloom import BloomFilter

        bloom = BloomFilter(expected_items=500, false_positive_rate=0.05)

        assert bloom is not None
        # Larger expected_items or lower FP rate = larger size
        bloom_small = BloomFilter(expected_items=100, false_positive_rate=0.1)
        assert bloom.size > bloom_small.size


class TestBloomFilterAdd:
    """Test adding items to bloom filter."""

    def test_add_item_and_check_membership(self):
        """Added items should test as 'probably in set'.

        RED: Run before implementation - should fail
        GREEN: Implement add() and __contains__()
        """
        from context_os_events.index.bloom import BloomFilter

        bloom = BloomFilter()
        bloom.add("/src/main.py")

        assert "/src/main.py" in bloom

    def test_non_added_items_probably_not_in_set(self):
        """Items never added should usually test as 'not in set'.

        Note: Bloom filters have false positives, so this test uses
        items very unlikely to collide.
        """
        from context_os_events.index.bloom import BloomFilter

        bloom = BloomFilter(expected_items=100, false_positive_rate=0.01)
        bloom.add("/src/main.py")

        # These should NOT be in the bloom filter
        assert "/totally/different/path.py" not in bloom
        assert "xyz123_unlikely_string" not in bloom

    def test_no_false_negatives(self):
        """Bloom filter NEVER has false negatives - added items always found.

        This is the key guarantee of bloom filters.
        """
        from context_os_events.index.bloom import BloomFilter

        bloom = BloomFilter(expected_items=1000, false_positive_rate=0.01)

        # Add many items
        items = [f"/src/file_{i}.py" for i in range(100)]
        for item in items:
            bloom.add(item)

        # ALL added items must be found
        for item in items:
            assert item in bloom, f"False negative: {item} not found"


class TestBloomFilterSerialization:
    """Test serialization for database storage."""

    def test_serialize_returns_bytes(self):
        """serialize() should return bytes for database storage.

        RED: Run before implementation - should fail
        GREEN: Implement serialize()
        """
        from context_os_events.index.bloom import BloomFilter

        bloom = BloomFilter()
        bloom.add("/src/main.py")

        data = bloom.serialize()

        assert isinstance(data, bytes)
        assert len(data) > 0

    def test_deserialize_restores_bloom_filter(self):
        """deserialize() should restore a working bloom filter.

        RED: Run before implementation - should fail
        GREEN: Implement deserialize()
        """
        from context_os_events.index.bloom import BloomFilter

        # Create and populate
        original = BloomFilter(expected_items=100, false_positive_rate=0.01)
        items = ["/src/a.py", "/src/b.py", "/src/c.py"]
        for item in items:
            original.add(item)

        # Serialize (self-describing format includes size/hash_count)
        data = original.serialize()

        # Deserialize (no extra params needed)
        restored = BloomFilter.deserialize(data)

        # Restored should contain all original items
        for item in items:
            assert item in restored, f"{item} not found after deserialize"

        # Should NOT contain items we didn't add
        assert "/totally/different.py" not in restored

    def test_serialize_deserialize_roundtrip(self):
        """Serialization roundtrip should preserve all state."""
        from context_os_events.index.bloom import BloomFilter

        original = BloomFilter(expected_items=500, false_positive_rate=0.01)

        # Add 100 items
        items = [f"/project/module_{i}/file_{i}.py" for i in range(100)]
        for item in items:
            original.add(item)

        # Roundtrip (self-describing format)
        data = original.serialize()
        restored = BloomFilter.deserialize(data)

        # All items must be found
        for item in items:
            assert item in restored

        # Verify metadata preserved
        assert restored.size == original.size
        assert restored.hash_count == original.hash_count


class TestBloomFilterOptimalSizing:
    """Test optimal size calculations."""

    def test_optimal_size_increases_with_expected_items(self):
        """More expected items = larger bit array."""
        from context_os_events.index.bloom import BloomFilter

        small = BloomFilter(expected_items=100, false_positive_rate=0.01)
        large = BloomFilter(expected_items=10000, false_positive_rate=0.01)

        assert large.size > small.size

    def test_optimal_size_increases_with_lower_fp_rate(self):
        """Lower false positive rate = larger bit array."""
        from context_os_events.index.bloom import BloomFilter

        high_fp = BloomFilter(expected_items=1000, false_positive_rate=0.1)
        low_fp = BloomFilter(expected_items=1000, false_positive_rate=0.001)

        assert low_fp.size > high_fp.size


class TestBloomFilterFalsePositiveRate:
    """Test that false positive rate is within expected bounds."""

    def test_false_positive_rate_approximate(self):
        """False positive rate should be approximately as specified.

        With expected_items=1000 and fp_rate=0.01, we expect ~1% false positives
        when checking random strings not added to the filter.
        """
        from context_os_events.index.bloom import BloomFilter
        import random
        import string

        bloom = BloomFilter(expected_items=1000, false_positive_rate=0.01)

        # Add exactly expected_items
        for i in range(1000):
            bloom.add(f"added_item_{i}")

        # Check 10000 random strings NOT added
        false_positives = 0
        test_count = 10000

        random.seed(42)
        for _ in range(test_count):
            # Generate random string unlikely to match added items
            random_str = "not_added_" + "".join(
                random.choices(string.ascii_letters + string.digits, k=20)
            )
            if random_str in bloom:
                false_positives += 1

        # Allow 2x the expected rate (statistical variance)
        # Expected: 1% = 100 false positives out of 10000
        # Allow up to: 2% = 200 false positives
        actual_fp_rate = false_positives / test_count
        assert actual_fp_rate < 0.02, f"FP rate {actual_fp_rate:.2%} too high (expected < 2%)"


class TestBloomFilterEdgeCases:
    """Test edge cases."""

    def test_empty_bloom_filter(self):
        """Empty bloom filter should contain nothing."""
        from context_os_events.index.bloom import BloomFilter

        bloom = BloomFilter()

        assert "/any/path.py" not in bloom
        assert "" not in bloom

    def test_empty_string(self):
        """Empty string can be added and checked."""
        from context_os_events.index.bloom import BloomFilter

        bloom = BloomFilter()
        bloom.add("")

        assert "" in bloom

    def test_unicode_strings(self):
        """Unicode strings should work correctly."""
        from context_os_events.index.bloom import BloomFilter

        bloom = BloomFilter()
        bloom.add("/src/日本語.py")
        bloom.add("/src/файл.py")

        assert "/src/日本語.py" in bloom
        assert "/src/файл.py" in bloom

    def test_very_long_strings(self):
        """Very long strings should work correctly."""
        from context_os_events.index.bloom import BloomFilter

        bloom = BloomFilter()
        long_path = "/src/" + "a" * 10000 + ".py"
        bloom.add(long_path)

        assert long_path in bloom
