extern crate winapi;
use std::ptr::null_mut;
use winapi::shared::windef::*;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

pub fn draw_focus_rectangle(rect: &RECT) {
    unsafe {
        let hdc = GetDC(null_mut()); // Get the device context for the entire screen
        let pen = CreatePen(
            PS_SOLID.try_into().unwrap(),
            3,              // 3px width
            RGB(255, 0, 0), // Red color
        );
        let old_pen = SelectObject(hdc, pen as _);

        // Create and select a null brush to make the rectangle transparent
        let null_brush = GetStockObject(NULL_BRUSH as i32);
        let old_brush = SelectObject(hdc, null_brush);

        // Draw the rectangle around the focused element
        Rectangle(hdc, rect.left, rect.top, rect.right, rect.bottom);

        // Cleanup
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        DeleteObject(pen as _);
        ReleaseDC(null_mut(), hdc);
    }
}
