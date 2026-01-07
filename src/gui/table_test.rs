use super::*;

#[test]
fn test_select_cell_behavior() {
    // Create a simple TableState
    let mut table_state = TableState {
        selected_cell: None,
        data: BTreeMap::new(),
        dates: Vec::new(),
        places: BTreeSet::new(),
    };
    
    // Test selecting a cell
    let pos = CellPosition { row: 1, column: 1 };
    let prev = table_state.select_cell(pos);
    
    // Should return None (no previous selection) and set the selected cell
    assert!(prev.is_none());
    assert_eq!(table_state.selected_cell, Some(pos));
    
    // Test selecting the same cell again (should deselect)
    let prev = table_state.select_cell(pos);
    
    // Should return the previous selection and clear the selected cell
    assert_eq!(prev, Some(pos));
    assert!(table_state.selected_cell.is_none());
}
