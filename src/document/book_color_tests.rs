use super::*;
use crate::objects::{BookColor, ObjectType};

/// Two book colors of one name resolve to the lowest handle, whatever order the objects map iterates in: an entity that names a book
/// color without a handle used to get either, so the same file saved twice differed.
#[test]
fn a_book_color_of_two_with_one_name_resolves_to_the_lowest_handle() {
    for _ in 0..32 {
        let mut doc = CadDocument::new();
        for handle in [0x200u64, 0x100, 0x300, 0x180] {
            let mut color = BookColor::new();
            color.handle = Handle::new(handle);
            color.book_name = "Book".into();
            color.color_name = "Red".into();
            doc.objects
                .insert(color.handle, ObjectType::BookColor(color));
        }
        let mut line = crate::entities::Line::new();
        line.common.color_name = Some("Book$Red".into());
        let handle = doc.add_entity(EntityType::Line(line)).unwrap();
        doc.resolve_book_colors();
        assert_eq!(
            doc.get_entity(handle).unwrap().common().color_book_handle,
            Some(Handle::new(0x100))
        );
    }
}
