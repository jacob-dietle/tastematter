"""Bloom filter implementation for Context OS Intelligence.

Pure Python implementation using hashlib (no external dependencies).
Used for fast probabilistic "is X in set?" checks.

Key properties:
- False positives possible (says "yes" when item wasn't added)
- False negatives impossible (never says "no" for added items)
- O(1) add and membership check
- Serializable for database storage

Use case: "Did chain X touch file Y?" - fast filtering before expensive DB queries.
"""

import hashlib
import math
from typing import Optional


class BloomFilter:
    """Probabilistic set membership data structure.

    Space-efficient way to test if an item is "probably in set" or "definitely not in set".

    Algorithm:
    1. Uses k different hash functions (simulated via seed)
    2. Each hash maps to a bit position in the bit array
    3. add(): Set all k bits to 1
    4. __contains__(): Check if all k bits are 1
    """

    def __init__(
        self,
        expected_items: int = 1000,
        false_positive_rate: float = 0.01
    ):
        """Create a new bloom filter.

        Args:
            expected_items: Expected number of items to add (affects size)
            false_positive_rate: Desired false positive rate (0.01 = 1%)
        """
        # Calculate optimal size and hash count
        self.size = self._optimal_size(expected_items, false_positive_rate)
        self.hash_count = self._optimal_hash_count(self.size, expected_items)

        # Bit array as bytearray (each byte holds 8 bits)
        self._byte_count = (self.size + 7) // 8  # Ceiling division
        self._bits = bytearray(self._byte_count)

    def add(self, item: str) -> None:
        """Add an item to the bloom filter.

        After adding, item will always test as "in set" (no false negatives).

        Args:
            item: String to add to the filter
        """
        for index in self._get_hash_indices(item):
            byte_index = index // 8
            bit_index = index % 8
            self._bits[byte_index] |= (1 << bit_index)

    def __contains__(self, item: str) -> bool:
        """Test if item is probably in the set.

        Returns:
            True: Item is PROBABLY in set (may be false positive)
            False: Item is DEFINITELY NOT in set (no false negatives)
        """
        for index in self._get_hash_indices(item):
            byte_index = index // 8
            bit_index = index % 8
            if not (self._bits[byte_index] & (1 << bit_index)):
                return False  # Definitely not in set
        return True  # Probably in set

    def serialize(self) -> bytes:
        """Serialize bloom filter for database storage.

        Format: [size:4 bytes][hash_count:4 bytes][bits...]
        Self-describing so deserialize doesn't need extra params.

        Returns:
            Bytes representation including metadata
        """
        # Pack size and hash_count as 4-byte big-endian integers
        header = self.size.to_bytes(4, 'big') + self.hash_count.to_bytes(4, 'big')
        return header + bytes(self._bits)

    @classmethod
    def deserialize(cls, data: bytes) -> "BloomFilter":
        """Restore bloom filter from serialized data.

        Args:
            data: Bytes from serialize() (self-describing format)

        Returns:
            Restored BloomFilter
        """
        if len(data) < 8:
            raise ValueError("Invalid bloom filter data: too short")

        # Unpack header
        size = int.from_bytes(data[0:4], 'big')
        hash_count = int.from_bytes(data[4:8], 'big')
        bits_data = data[8:]

        # Create instance without normal __init__
        instance = cls.__new__(cls)
        instance.size = size
        instance.hash_count = hash_count
        instance._byte_count = len(bits_data)
        instance._bits = bytearray(bits_data)
        return instance

    def _get_hash_indices(self, item: str) -> list:
        """Get hash indices for an item using double hashing.

        Uses double hashing technique: h(i) = (h1 + i * h2) % size
        This gives k different hash values from two hash functions.

        Args:
            item: String to hash

        Returns:
            List of k bit indices
        """
        # Get two independent hashes
        item_bytes = item.encode("utf-8")

        # Hash 1: SHA-256 of item
        h1 = int(hashlib.sha256(item_bytes).hexdigest(), 16)

        # Hash 2: SHA-256 of item + salt
        h2 = int(hashlib.sha256(item_bytes + b"_bloom_salt").hexdigest(), 16)

        # Generate k indices using double hashing
        indices = []
        for i in range(self.hash_count):
            index = (h1 + i * h2) % self.size
            indices.append(index)

        return indices

    @staticmethod
    def _optimal_size(n: int, p: float) -> int:
        """Calculate optimal bit array size.

        Formula: m = -n * ln(p) / (ln(2)^2)

        Args:
            n: Expected number of items
            p: Desired false positive rate

        Returns:
            Optimal bit array size
        """
        if n <= 0:
            return 64  # Minimum size
        if p <= 0 or p >= 1:
            p = 0.01  # Default to 1%

        m = -n * math.log(p) / (math.log(2) ** 2)
        return max(64, int(m))  # Minimum 64 bits

    @staticmethod
    def _optimal_hash_count(m: int, n: int) -> int:
        """Calculate optimal number of hash functions.

        Formula: k = (m/n) * ln(2)

        Args:
            m: Bit array size
            n: Expected number of items

        Returns:
            Optimal number of hash functions
        """
        if n <= 0:
            return 7  # Default

        k = (m / n) * math.log(2)
        return max(1, int(k))  # At least 1 hash function
