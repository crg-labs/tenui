use tenui_core::Rect;

/// Decomposes `subject` by subtracting `cutter` (i.e. `subject \ cutter`),
/// returning 0 to 4 non-overlapping rectangular fragments.
pub fn cut_rect(subject: Rect, cutter: Rect) -> Vec<Rect> {
    if subject.is_empty() {
        return Vec::new();
    }
    if !subject.intersects(&cutter) {
        return vec![subject];
    }

    let mut fragments = Vec::with_capacity(4);

    // 1. Top fragment
    if cutter.y > subject.y {
        let top_h = cutter.y - subject.y;
        fragments.push(Rect::new(subject.x, subject.y, subject.width, top_h));
    }

    // 2. Bottom fragment
    if cutter.bottom() < subject.bottom() {
        let bot_y = cutter.bottom();
        let bot_h = subject.bottom() - cutter.bottom();
        fragments.push(Rect::new(subject.x, bot_y, subject.width, bot_h));
    }

    // Overlapping Y interval for left and right fragments
    let mid_top = subject.y.max(cutter.y);
    let mid_bottom = subject.bottom().min(cutter.bottom());

    if mid_bottom > mid_top {
        let mid_h = mid_bottom - mid_top;

        // 3. Left fragment
        if cutter.x > subject.x {
            let left_w = cutter.x - subject.x;
            fragments.push(Rect::new(subject.x, mid_top, left_w, mid_h));
        }

        // 4. Right fragment
        if cutter.right() < subject.right() {
            let right_x = cutter.right();
            let right_w = subject.right() - cutter.right();
            fragments.push(Rect::new(right_x, mid_top, right_w, mid_h));
        }
    }

    fragments
}

/// Subtracts a list of obscuring rects from a target rect, returning all visible fragments.
pub fn cut_multi(target: Rect, cutters: &[Rect]) -> Vec<Rect> {
    let mut current_fragments = vec![target];

    for &cutter in cutters {
        let mut next_fragments = Vec::new();
        for frag in current_fragments {
            let pieces = cut_rect(frag, cutter);
            next_fragments.extend(pieces);
        }
        current_fragments = next_fragments;
        if current_fragments.is_empty() {
            break;
        }
    }

    current_fragments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cut_no_overlap_returns_subject() {
        let subject = Rect::new(0, 0, 10, 10);
        let cutter = Rect::new(20, 20, 5, 5);
        let frags = cut_rect(subject, cutter);
        assert_eq!(frags, vec![subject]);
    }

    #[test]
    fn cut_full_overlap_returns_empty() {
        let subject = Rect::new(5, 5, 10, 10);
        let cutter = Rect::new(0, 0, 20, 20);
        let frags = cut_rect(subject, cutter);
        assert!(frags.is_empty());
    }

    #[test]
    fn cut_center_produces_four_fragments() {
        let subject = Rect::new(0, 0, 10, 10);
        let cutter = Rect::new(3, 3, 4, 4);
        let frags = cut_rect(subject, cutter);
        assert_eq!(frags.len(), 4);
    }

    #[test]
    fn cut_left_edge_produces_three_fragments() {
        let subject = Rect::new(0, 0, 10, 10);
        let cutter = Rect::new(0, 3, 4, 4);
        let frags = cut_rect(subject, cutter);
        assert_eq!(frags.len(), 3);
        assert!(frags.iter().all(|f| f.x >= cutter.right() || f.y < cutter.y || f.y >= cutter.bottom()));
    }

    #[test]
    fn cut_top_edge_produces_fragments_without_top() {
        let subject = Rect::new(0, 0, 10, 10);
        let cutter = Rect::new(3, 0, 4, 4);
        let frags = cut_rect(subject, cutter);
        assert!(!frags.is_empty());
        assert!(frags.iter().all(|f| f.y >= cutter.bottom() || f.x < cutter.x || f.x >= cutter.right()));
    }

    #[test]
    fn cut_empty_subject_returns_empty() {
        let subject = Rect::ZERO;
        let cutter = Rect::new(0, 0, 5, 5);
        let frags = cut_rect(subject, cutter);
        assert!(frags.is_empty());
    }

    #[test]
    fn cut_fragments_do_not_overlap() {
        let subject = Rect::new(0, 0, 20, 20);
        let cutter = Rect::new(5, 5, 10, 10);
        let frags = cut_rect(subject, cutter);
        for i in 0..frags.len() {
            for j in (i + 1)..frags.len() {
                assert!(
                    !frags[i].intersects(&frags[j]),
                    "fragments {i} and {j} overlap: {:?} vs {:?}",
                    frags[i],
                    frags[j]
                );
            }
        }
    }

    #[test]
    fn cut_area_conservation() {
        let subject = Rect::new(0, 0, 20, 20);
        let cutter = Rect::new(5, 5, 10, 10);
        let frags = cut_rect(subject, cutter);
        let frag_area: u32 = frags.iter().map(|f| f.area()).sum();
        let intersection = subject.intersection(&cutter);
        assert_eq!(frag_area, subject.area() - intersection.area());
    }

    #[test]
    fn cut_exact_match_returns_empty() {
        let r = Rect::new(5, 5, 10, 10);
        let frags = cut_rect(r, r);
        assert!(frags.is_empty());
    }

    #[test]
    fn cut_cutter_wider_than_subject() {
        let subject = Rect::new(5, 5, 10, 10);
        let cutter = Rect::new(0, 7, 20, 3);
        let frags = cut_rect(subject, cutter);
        let frag_area: u32 = frags.iter().map(|f| f.area()).sum();
        let intersection = subject.intersection(&cutter);
        assert_eq!(frag_area, subject.area() - intersection.area());
    }

    #[test]
    fn cut_multi_empty_cutters_returns_subject() {
        let target = Rect::new(0, 0, 10, 10);
        let frags = cut_multi(target, &[]);
        assert_eq!(frags, vec![target]);
    }

    #[test]
    fn cut_multi_fully_obscured() {
        let target = Rect::new(5, 5, 10, 10);
        let cutters = vec![Rect::new(0, 0, 20, 20)];
        let frags = cut_multi(target, &cutters);
        assert!(frags.is_empty());
    }

    #[test]
    fn cut_multi_two_cutters() {
        let target = Rect::new(0, 0, 30, 10);
        let cutters = vec![Rect::new(5, 0, 5, 10), Rect::new(20, 0, 5, 10)];
        let frags = cut_multi(target, &cutters);
        let frag_area: u32 = frags.iter().map(|f| f.area()).sum();
        assert_eq!(frag_area, 30 * 10 - 5 * 10 - 5 * 10);
    }
}
