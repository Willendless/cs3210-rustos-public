use hashbrown::HashMap;
use core::fmt::Debug;

#[derive(Default)]
pub struct Entry<V: Default + Debug> {
    key: u64,
    val: V,
    prev: u64,
    next: u64,
}

pub struct LRUCache<V: Default + Debug> {
    entries: [Entry<V>; 64],
    head: u64,
    tail: u64,
    next_empty: u64,
    map: HashMap<u64, u64>,
}

impl<V: Default + Debug> Entry<V> {
    fn new(key: u64, val: V, prev: u64, next: u64) -> Entry<V> {
        Entry {
            key,
            val,
            prev,
            next,
        }
    }
}

impl<V: Default + Debug> LRUCache<V> {
    const MAX_SIZE: usize = 64;

    pub fn head(&mut self) -> u64 {
        self.head
    }

    pub fn new() -> LRUCache<V> {
        LRUCache {
            entries: [Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            Entry::<V>::new(0, Default::default(), 0, 0),
            ],
            head: 0,
            tail: 0,
            next_empty: 0,
            map: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.len() == 0
    }

    pub fn is_full(&self) -> bool {
        self.map.len() == Self::MAX_SIZE
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
        self.map.clear();
    }

    pub fn contains_key(&self, key: &u64) -> bool {
        self.map.contains_key(key)
    }

    pub fn get(&mut self, key: u64) -> Option<&V> {
        match self.map.get(&key) {
            Some(index) => {
                if self.tail == *index {
                    self.tail = self.entries[*index as usize].prev;
                    self.head = *index;
                    return Some(&self.entries[*index as usize].val);
                }
                let hit_entry = &self.entries[*index as usize];
                let hit_entry_prev = hit_entry.prev;
                let hit_entry_next = hit_entry.next;
                self.entries[hit_entry_prev as usize].next = hit_entry_next;
                self.entries[hit_entry_next as usize].prev = hit_entry_prev;
                self.entries[*index as usize].prev = self.tail;
                self.entries[*index as usize].next = self.head;
                self.head = *index;
                Some(&self.entries[*index as usize].val)
            },
            None => None
        }
    }

    pub fn get_mut(&mut self, key: u64) -> Option<&mut V> {
        match self.map.get(&key) {
            Some(index) => {
                if self.tail == *index {
                    self.tail = self.entries[*index as usize].prev;
                    self.head = *index;
                    return Some(&mut self.entries[*index as usize].val);
                }
                let hit_entry = &self.entries[*index as usize];
                let hit_entry_prev = hit_entry.prev;
                let hit_entry_next = hit_entry.next;
                self.entries[hit_entry_prev as usize].next = hit_entry_next;
                self.entries[hit_entry_next as usize].prev = hit_entry_prev;
                self.entries[*index as usize].prev = self.tail;
                self.entries[*index as usize].next = self.head;
                self.head = *index;
                Some(&mut self.entries[*index as usize].val)
            },
            None => None
        }
    }

    pub fn put(&mut self, key: u64, val: V) {
        if self.map.contains_key(&key) { 
            // index = self.map.get(&key).unwrap();
            // self.entries[index].val = val;
            return;
        }
        if self.is_full() {
            self.map.remove_entry(&self.entries[self.tail as usize].key);
            self.ll_push_full(key, val);
        } else {
            self.ll_push_not_full(key, val);
        }
        self.map.insert(key, self.head);
    }

    #[inline(always)]
    fn ll_push_full(&mut self, key: u64, val: V) {
        // when the ll is full, push according to tail
        let next_head = self.tail;
        let next_tail = self.entries[self.tail as usize].prev;
        self.entries[next_head as usize] = Entry::new(key, val, next_tail, self.head);
        self.head = next_head;
        self.tail = next_tail;
        // println!("push full, head: {}, tail: {}", next_head, next_tail);
    }

    #[inline(always)]
    fn ll_push_not_full(&mut self, key: u64, val: V) {
        // when the ll is not full, push according to head
        if self.is_empty() {
            self.entries[self.next_empty as usize] = Entry::new(key, val, 0, 0);
            self.head = 0;
            self.tail = 0;
            self.next_empty += 1;
        } else {
            self.entries[self.next_empty as usize] = Entry::new(key, val, 0, self.next_empty);
            self.entries[self.head as usize].prev = self.next_empty;
            self.entries[self.tail as usize].next = self.next_empty;
            self.head = self.next_empty;
            self.next_empty += 1;
        }
    }
}
