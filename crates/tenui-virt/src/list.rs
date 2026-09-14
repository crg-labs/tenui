#[derive(Clone, Debug)]
pub struct FenwickTree {
    tree: Vec<u32>,
    size: usize,
}

impl FenwickTree {
    pub fn new(size: usize, default_height: u32) -> Self {
        let mut ft = Self {
            tree: vec![0; size + 1],
            size,
        };
        for i in 1..=size {
            ft.add(i, default_height);
        }
        ft
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn add(&mut self, mut idx: usize, delta: u32) {
        while idx <= self.size {
            self.tree[idx] += delta;
            idx += idx & (!idx + 1); // idx += LSB(idx)
        }
    }

    pub fn update(&mut self, idx: usize, old_val: u32, new_val: u32) {
        if new_val >= old_val {
            self.add(idx, new_val - old_val);
        } else {
            let diff = old_val - new_val;
            let mut i = idx;
            while i <= self.size {
                self.tree[i] -= diff;
                i += i & (!i + 1);
            }
        }
    }

    pub fn prefix_sum(&self, mut idx: usize) -> u32 {
        let mut sum = 0;
        idx = idx.min(self.size);
        while idx > 0 {
            sum += self.tree[idx];
            idx -= idx & (!idx + 1);
        }
        sum
    }

    /// Binary search for largest index with prefix_sum <= target_y: O(log N)
    pub fn find_index_at_offset(&self, target_y: u32) -> usize {
        let mut idx = 0;
        if self.size == 0 {
            return 0;
        }
        let mut bit_mask = 1 << (usize::BITS - 1 - self.size.leading_zeros());
        let mut current_sum = 0;

        while bit_mask != 0 {
            let next_idx = idx + bit_mask;
            if next_idx <= self.size && current_sum + self.tree[next_idx] <= target_y {
                idx = next_idx;
                current_sum += self.tree[next_idx];
            }
            bit_mask >>= 1;
        }
        idx.min(self.size)
    }
}

#[derive(Clone, Debug)]
pub struct VirtualList<T> {
    pub items: Vec<T>,
    height_tree: FenwickTree,
    cached_heights: Vec<u32>,
    pub scroll_offset: f32,
    anchor_id: usize,
    anchor_sub_offset: f32,
}

impl<T> VirtualList<T> {
    pub fn new(items: Vec<T>, default_height: u32) -> Self {
        let len = items.len();
        Self {
            items,
            height_tree: FenwickTree::new(len, default_height),
            cached_heights: vec![default_height; len],
            scroll_offset: 0.0,
            anchor_id: 0,
            anchor_sub_offset: 0.0,
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn total_height(&self) -> u32 {
        self.height_tree.prefix_sum(self.items.len())
    }

    pub fn set_item_height(&mut self, item_index: usize, new_height: u32) {
        if item_index >= self.items.len() {
            return;
        }
        let old_height = self.cached_heights[item_index];
        if old_height != new_height {
            self.cached_heights[item_index] = new_height;
            self.height_tree.update(item_index + 1, old_height, new_height);

            // Maintain continuous scroll anchoring if height changes above current viewport anchor
            if item_index < self.anchor_id {
                let delta = new_height as i32 - old_height as i32;
                self.scroll_offset = (self.scroll_offset + delta as f32).max(0.0);
            }
        }
    }

    pub fn scroll(&mut self, delta: f32) {
        let max_scroll = (self.total_height() as f32 - 1.0).max(0.0);
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max_scroll);
    }

    pub fn resolve_visible_range(&mut self, viewport_h: u32) -> (usize, usize, f32) {
        if self.items.is_empty() || viewport_h == 0 {
            return (0, 0, 0.0);
        }

        let scroll_y = self.scroll_offset.floor() as u32;
        let start_idx = self.height_tree.find_index_at_offset(scroll_y);
        let start_y = self.height_tree.prefix_sum(start_idx);

        let sub_offset = self.scroll_offset - start_y as f32;

        let end_y = scroll_y + viewport_h;
        let end_idx = (self.height_tree.find_index_at_offset(end_y) + 1).min(self.items.len());

        self.anchor_id = start_idx;
        self.anchor_sub_offset = sub_offset;

        (start_idx, end_idx, sub_offset)
    }

    pub fn anchor_id(&self) -> usize {
        self.anchor_id
    }

    pub fn anchor_sub_offset(&self) -> f32 {
        self.anchor_sub_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fenwick_tree_prefix_and_binary_search() {
        // 5 items with dynamic heights: [3, 1, 8, 4, 2]
        let heights = [3, 1, 8, 4, 2];
        let mut ft = FenwickTree::new(5, 0);
        for (i, &h) in heights.iter().enumerate() {
            ft.add(i + 1, h);
        }

        assert_eq!(ft.prefix_sum(1), 3);
        assert_eq!(ft.prefix_sum(3), 12); // 3 + 1 + 8 = 12
        assert_eq!(ft.prefix_sum(5), 18); // Total vertical height

        // Seek offset y = 10 -> falls inside item 3 (prefix_sum(2)=4, prefix_sum(3)=12)
        let idx = ft.find_index_at_offset(10);
        assert_eq!(idx, 2);
    }

    #[test]
    fn test_fenwick_update() {
        let mut ft = FenwickTree::new(4, 2); // all items height 2: [2, 2, 2, 2] -> total 8
        assert_eq!(ft.prefix_sum(4), 8);

        ft.update(2, 2, 5); // item 2 grows from 2 to 5 -> total 11
        assert_eq!(ft.prefix_sum(1), 2);
        assert_eq!(ft.prefix_sum(2), 7);
        assert_eq!(ft.prefix_sum(4), 11);

        ft.update(2, 5, 1); // item 2 shrinks from 5 to 1 -> total 7
        assert_eq!(ft.prefix_sum(2), 3);
        assert_eq!(ft.prefix_sum(4), 7);
    }

    #[test]
    fn test_virtual_list_scroll_anchoring() {
        let items: Vec<String> = (0..100).map(|i| format!("Item {}", i)).collect();
        let mut list = VirtualList::new(items, 3); // 100 items, each 3 rows

        list.scroll_offset = 30.0; // Viewport begins around item 10
        let (start_idx, end_idx, sub) = list.resolve_visible_range(10);
        assert_eq!(start_idx, 10);
        assert_eq!(sub, 0.0);
        assert!(end_idx > start_idx);

        // Resize an item ABOVE the active viewport anchor (e.g. item 2 grows by 4 rows)
        list.set_item_height(2, 7); // old was 3, new is 7 (delta +4)
        assert_eq!(list.scroll_offset, 34.0); // Scroll anchored: offset pushed down by 4, viewport content locked!
    }
}
