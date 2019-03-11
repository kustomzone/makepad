use render::*;

use crate::scrollbar::*;
use crate::tab::*;

#[derive(Clone, Element)]
pub struct TabControl{
    pub tab_dock_height:f32,

    pub tabs_view:View<ScrollBar>,
    pub tabs:Elements<Tab, usize>,
    pub drag_tab_view:View<NoScrollBar>,
    pub drag_tab:Element<Tab>,
    pub hover:Quad,
    pub animator:Animator,

    pub _dragging_tab:Option<(FingerMoveEvent,usize)>,
    pub _tab_id_alloc:usize,
    pub _page_rect:Rect
}

#[derive(Clone, PartialEq)]
pub enum TabControlEvent{
    None,
    TabDragMove{fe:FingerMoveEvent, tab_id:usize},
    TabDragEnd{fe:FingerUpEvent, tab_id:usize},
    TabSelect{tab_id:usize},
}

impl Style for TabControl{
    fn style(cx:&mut Cx)->Self{
        Self{
            tab_dock_height:30.0,

            tabs_view:View{
                scroll_h:Some(Element::new(ScrollBar{
                    bar_size:4.0,
                    ..Style::style(cx)
                })),
                ..Style::style(cx)
            },
            tabs:Elements::new(Tab{
                ..Style::style(cx)
            }),
            drag_tab:Element::new(Tab{
                ..Style::style(cx)
            }),
            drag_tab_view:View{
                is_overlay:true,
                ..Style::style(cx)
            },
            hover:Quad{
                color:color("purple"),
                ..Style::style(cx)
            },
            animator:Animator::new(Anim::new(AnimMode::Cut{duration:0.5}, vec![])),
            _dragging_tab:None,
            _tab_id_alloc:0,
            _page_rect:Rect::zero()
        }
    }
}

impl TabControl{
    pub fn handle_tab_control(&mut self, cx:&mut Cx, event:&mut Event)->TabControlEvent{
        for (id, tab) in self.tabs.ids(){
            match tab.handle_tab(cx, event){
                TabEvent::Clicked=>{

                },
                TabEvent::DragMove(fe)=>{
                    self._dragging_tab = Some((fe.clone(), *id));
                    // flag our view as dirty, to trigger
                    cx.redraw_area(self.tabs_view.get_view_area());
                    return TabControlEvent::TabDragMove{fe:fe, tab_id:*id};
                },
                TabEvent::DragEnd(fe)=>{
                    self._dragging_tab = None;
                    cx.redraw_area(self.tabs_view.get_view_area());
                    return TabControlEvent::TabDragEnd{fe, tab_id:*id};
                }
                _=>()
            }
        }
        TabControlEvent::None
    }

    pub fn get_tab_drop_rect(&mut self, cx:&Cx)->Rect{
        self.tabs_view.get_view_area().get_rect(cx, true)
    }

    pub fn get_content_drop_rect(&mut self, cx:&Cx)->Rect{
        let rc = self.tabs_view.get_view_area().get_rect(cx, true);
        // we now need to change the y and the new height
        Rect{
            x:rc.x,
            y:rc.y + rc.h,
            w:self._page_rect.w,
            h:self._page_rect.h
        }
    }

    // data free APIs for the win!
    pub fn begin_tabs(&mut self, cx:&mut Cx){
        //cx.begin_turtle(&Layout{
        self.tabs_view.begin_view(cx, &Layout{
            width:Bounds::Fill,
            height:Bounds::Compute,
           ..Default::default()
        });
        //self.tabs.mark();
        self._tab_id_alloc = 0;
    }

    pub fn draw_tab(&mut self, cx:&mut Cx, label:&str, selected:bool){
        let tab = self.tabs.get(cx, self._tab_id_alloc);
        self._tab_id_alloc += 1;
        tab.label = label.to_string();
        tab.draw_tab(cx);
    }

    pub fn end_tabs(&mut self, cx:&mut Cx){
        self.tabs_view.end_view(cx);

        if let Some((fe, id)) = &self._dragging_tab{
            self.drag_tab_view.begin_view(cx, &Layout{
                abs_x:Some(0.),
                abs_y:Some(0.),
                ..Default::default()
            });
            
            let drag_tab = self.drag_tab.get(cx);
            drag_tab.bg_layout.abs_x = Some(fe.abs_x - fe.rel_start_x);
            drag_tab.bg_layout.abs_y = Some(fe.abs_y - fe.rel_start_y);

            let origin_tab = self.tabs.get(cx, *id);
            drag_tab.label = origin_tab.label.clone();

            drag_tab.draw_tab(cx);

            self.drag_tab_view.end_view(cx);
        }
    }

    pub fn begin_tab_page(&mut self, cx:&mut Cx){
        cx.turtle_new_line();
        cx.begin_turtle(&Layout{..Default::default()}, Area::Empty);
        self._page_rect = cx.turtle_rect();
    }

    pub fn end_tab_page(&mut self, cx:&mut Cx){
        cx.end_turtle(Area::Empty);
        // if we are in draggable tab state,
        // draw our draggable tab
    }
}
