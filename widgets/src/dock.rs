use std::mem;

use render::*;
use crate::splitter::*;
use crate::tabcontrol::*;

#[derive(Clone)]
pub struct Dock<TItem>
where TItem: Clone
{
    pub dock_items: Option<DockItem<TItem>>,
    pub splitters: Elements<Splitter, usize>,
    pub tab_controls: Elements<TabControl, usize>,

    pub drop_quad: Quad,
    pub drop_quad_view:View<NoScrollBar>,
    pub _drop_quad_where: Option<FingerMoveEvent>
}

impl<TItem> Style for Dock<TItem>
where TItem: Clone
{
    fn style(cx: &mut Cx)->Dock<TItem>{
        Dock{
            dock_items:None,
            drop_quad:Quad{
                color:cx.style_color("accent_normal"),
                ..Style::style(cx)
            },
            splitters:Elements::new(Splitter{
                ..Style::style(cx)
            }),
            tab_controls:Elements::new(TabControl{
                ..Style::style(cx)
            }),
            drop_quad_view:View{
                is_overlay:true,
                ..Style::style(cx)
            },
            _drop_quad_where:None
        }
    }
}

#[derive(Clone)]
pub struct DockTab<TItem>
where TItem: Clone
{
    pub title:String,
    pub item:TItem
}

#[derive(Clone)]
pub enum DockItem<TItem>
where TItem: Clone
{
    Single(TItem),
    TabControl{
        current:usize,
        tabs:Vec<DockTab<TItem>>,
    },
    Splitter{
        align:SplitterAlign,
        pos:f32,
        axis:Axis,
        first:Box<DockItem<TItem>>, 
        last:Box<DockItem<TItem>>
    }
}

struct DockWalkStack<'a, TItem>
where TItem: Clone
{
    counter:usize,
    uid:usize,
    item:&'a mut DockItem<TItem>
}

pub struct DockWalker<'a, TItem>
where TItem: Clone
{
    walk_uid:usize,
    stack:Vec<DockWalkStack<'a, TItem>>,
    // forwards for Dock
    splitters:&'a mut Elements<Splitter, usize>,
    tab_controls:&'a mut Elements<TabControl, usize>,
    drop_quad_view:&'a mut View<NoScrollBar>,
    _drop_quad_where:&'a mut Option<FingerMoveEvent>
}

impl<'a, TItem> DockWalker<'a, TItem>
where TItem: Clone
{
    pub fn walk_handle_dock(&mut self, cx: &mut Cx, event: &mut Event)->Option<&mut TItem>{
        // lets get the current item on the stack
        let push_or_pop = if let Some(stack_top) = self.stack.last_mut(){
            // return item 'count'
            match stack_top.item{
                DockItem::Single(item)=>{
                    if stack_top.counter == 0{
                        stack_top.counter += 1;
                        return Some(unsafe{mem::transmute(item)});
                    }
                    else{
                        None
                    }
                },
                DockItem::TabControl{current, tabs}=>{
                    if stack_top.counter == 0{
                        stack_top.counter += 1;
                        stack_top.uid = self.walk_uid;
                        self.walk_uid += 1;
                        let tab_control = self.tab_controls.get(cx, stack_top.uid);
                        
                        // ok so this one returns 'DragTab(x,y)
                        match tab_control.handle_tab_control(cx, event){
                            TabControlEvent::TabDragMove{fe, tab_id}=>{
                               *self._drop_quad_where = Some(fe);
                               self.drop_quad_view.redraw_view_area(cx);
                            },
                            //TabControlEvent::TabDragEnd{_fe, _tab_id}=>{
                            //}
                            _=>()
                        }

                        return Some(unsafe{mem::transmute(&mut tabs[*current].item)});
                    }
                    else{
                        None
                    }
                },
                DockItem::Splitter{first, last, pos, ..}=>{
                    if stack_top.counter == 0{
                        stack_top.counter += 1;
                        stack_top.uid = self.walk_uid;
                        self.walk_uid += 1;
                        let split = self.splitters.get(cx, stack_top.uid);
                        match split.handle_splitter(cx, event){
                            SplitterEvent::Moving{new_pos}=>{
                                *pos = new_pos;
                            },
                            _=>()
                        };
                        // update state in our splitter level
                        Some(DockWalkStack{counter:0, uid:0, item:unsafe{mem::transmute(first.as_mut())}})
                    }
                    else if stack_top.counter == 1{
                        stack_top.counter +=1;
                        Some(DockWalkStack{counter:0, uid:0, item:unsafe{mem::transmute(last.as_mut())}})
                    }
                    else{
                        None
                    }
                }
            }
        }
        else{
            return None;
        };
        if let Some(item) = push_or_pop{
            self.stack.push(item);
            return self.walk_handle_dock(cx, event);
        }
        else if self.stack.len() > 0{
            self.stack.pop();
            return self.walk_handle_dock(cx, event);
        }
        return None;
    }

    pub fn walk_draw_dock(&mut self, cx: &mut Cx)->Option<&'a mut TItem>{
        // lets get the current item on the stack
         let push_or_pop = if let Some(stack_top) = self.stack.last_mut(){
           
            // return item 'count'
            match stack_top.item{
                DockItem::Single(item)=>{
                    if stack_top.counter == 0{
                        stack_top.counter += 1;
                        return Some(unsafe{mem::transmute(item)});
                    }
                    else{
                        None
                    }
                },
                DockItem::TabControl{current, tabs}=>{
                    if stack_top.counter == 0{
                        stack_top.counter += 1;
                        stack_top.uid = self.walk_uid;
                        self.walk_uid += 1;
                        let tab_control = self.tab_controls.get(cx, stack_top.uid);
                        tab_control.begin_tabs(cx);
                        for tab in tabs.iter(){
                            tab_control.draw_tab(cx, &tab.title, false);
                        }
                        tab_control.end_tabs(cx);
                        tab_control.begin_tab_page(cx);
                        return Some(unsafe{mem::transmute(&mut tabs[*current].item)});
                    }
                    else{
                        let tab_control = self.tab_controls.get(cx, stack_top.uid);
                        tab_control.end_tab_page(cx);
                        None
                    }
                },
                DockItem::Splitter{align, pos, axis, first, last}=>{
                    if stack_top.counter == 0{
                        stack_top.counter += 1;
                        stack_top.uid = self.walk_uid;
                        self.walk_uid += 1;
                        // begin a split
                        let split = self.splitters.get(cx, stack_top.uid);
                        split.set_splitter_state(align.clone(), *pos, axis.clone());
                        split.begin_splitter(cx);
                        Some(DockWalkStack{counter:0, uid:0, item:unsafe{mem::transmute(first.as_mut())}})
                    }
                    else if stack_top.counter == 1{
                        stack_top.counter +=1 ;

                        let split = self.splitters.get(cx, stack_top.uid);
                        split.mid_splitter(cx);
                        Some(DockWalkStack{counter:0, uid:0, item:unsafe{mem::transmute(last.as_mut())}})
                    }
                    else{
                        let split = self.splitters.get(cx, stack_top.uid);
                        split.end_splitter(cx);
                        None
                    }
                }
            }
        }
        else{
            return None
        };
        if let Some(item) = push_or_pop{
            self.stack.push(item);
            return self.walk_draw_dock(cx);
        }
        else if self.stack.len() > 0{
            self.stack.pop();
            return self.walk_draw_dock(cx);
        }
        None
    }
}

impl<TItem> Dock<TItem>
where TItem: Clone
{
    pub fn draw_dock_drags(&mut self, cx: &mut Cx){
        // lets draw our hover layer if need be
        if let Some(fe) = &self._drop_quad_where{
            self.drop_quad_view.begin_view(cx, &Layout{
                abs_x:Some(0.),
                abs_y:Some(0.),
                ..Default::default()
            });

            // alright so now, what do i need to do
            // well lets for shits n giggles find all the tab areas 
            // you know, we have a list eh
            for tab_control in self.tab_controls.all(){
                // ok now, we ask the tab_controls rect
                let cdr = tab_control.get_content_drop_rect(cx);
                if cdr.contains(fe.abs_x, fe.abs_y){
                    self.drop_quad.draw_quad(cx, cdr.x, cdr.y, cdr.w, cdr.h);
                }
            }
            //self.drop_quad.draw_quad()

            self.drop_quad_view.end_view(cx);
        }
    }

    pub fn walker<'a>(&'a mut self)->DockWalker<'a, TItem>{
        let mut stack = Vec::new();
        if !self.dock_items.is_none(){
            stack.push(DockWalkStack{counter:0, uid:0, item:self.dock_items.as_mut().unwrap()});
        }
        DockWalker{
            walk_uid:0,
            stack:stack,
            splitters:&mut self.splitters,
            tab_controls:&mut self.tab_controls,
            _drop_quad_where:&mut self._drop_quad_where,
            drop_quad_view:&mut self.drop_quad_view,
        }
    }
}
