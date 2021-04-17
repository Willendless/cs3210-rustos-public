#![no_std]
use crate::lrucache::LRUCache;

#[test]
fn check_naive_get() {
    let mut lru = LRUCache::<&str>::new();
    lru.put(1, "abc");
    assert_eq!(*lru.get(1).unwrap(), "abc");
    lru.put(2, "cd");
    assert_eq!(*lru.get(2).unwrap(), "cd");
    lru.put(3, "fg");
    assert_eq!(*lru.get(3).unwrap(), "fg");
    lru.put(4, "I am a boy");
    assert_eq!(*lru.get(4).unwrap(), "I am a boy");
}

#[test]
fn check_evict() {
    let mut lru = LRUCache::<u64>::new();
    for i in 0..=67 {
        lru.put(i, i);
    }
    assert_eq!(lru.get(0), None);
    assert_eq!(lru.get(1), None);
    assert_eq!(lru.get(2), None);
    assert_eq!(lru.get(3), None);
    for i in 100..164 {
        lru.put(i, i);
        assert_eq!(lru.get(i).unwrap(), &i);
    }
    for i in 0..=67 {
        assert_eq!(lru.get(i), None);
    }
}

#[test]
fn check_get_and_evict() {
    let mut lru = LRUCache::<u64>::new();
    for i in 0..64 {
        lru.put(i, i);
    }
    for i in 0..64 {
        assert_eq!(*lru.get(i).unwrap(), i);
    }
    for i in 0..64 {
        lru.put(i, i);
    }
    for i in 0..64 {
        assert_eq!(*lru.get(i).unwrap(), i);
    }
    lru.get(0);
    lru.get(1);
    lru.get(2);
    lru.put(65, 65);
    lru.put(66, 66);
    lru.put(67, 67);
    assert_eq!(lru.get(3), None);
    assert_eq!(lru.get(4), None);
    assert_eq!(lru.get(5), None);
}

#[test]
fn test_multiple_same_block_access() {
    let mut lru = LRUCache::<u64>::new();
    for i in 0..32 {
        lru.put(i, i);
    }
    lru.get(1);
    lru.get(1);
    assert_eq!(lru.head(), 1);
    lru.get(2);
    assert_eq!(lru.head(), 2);
    lru.get(31);
    lru.get(2);
    lru.get(1);
    lru.get(10);
    lru.get(9);
    let cur = lru.head();
}


#[test]
fn test_get_before_full() {
    let mut lru = LRUCache::<u64>::new();
    lru.put(1, 1);
    lru.put(2, 2);
    lru.put(3, 3);
    assert_eq!(*lru.get(1).unwrap(), 1);
    lru.put(4, 4);
    assert_eq!(*lru.get(2).unwrap(), 2);
}
