use render::*;

#[derive(Clone, Element)]
pub struct Splitter{
    pub axis:Axis,
    pub align:SplitterAlign,
    pub pos:f32,

    pub min_size:f32,
    pub split_size:f32,
    pub split: Quad,
    pub animator:Animator,
    pub anim_over:Anim,
    pub anim_moving:Anim,

    pub _split_area:Area,
    pub _hit_state:HitState,
    pub _calc_pos:f32,
    pub _is_moving:bool,
    pub _drag_point:f32,
    pub _drag_pos_start:f32,
    pub _drag_max_pos:f32,

}

#[derive(Clone, PartialEq)]
pub enum SplitterAlign{
    First,
    Last,
    Weighted
}

#[derive(Clone, PartialEq)]
pub enum SplitterEvent{
    None,
    Moving{new_pos:f32},
}

impl Style for Splitter{
    fn style(cx:&mut Cx)->Self{
        let split_sh = Self::def_split_shader(cx);
        Self{

            axis:Axis::Vertical,
            align:SplitterAlign::First,
            pos:0.0,

            _split_area:Area::Empty,
            _hit_state:HitState{..Default::default()},
            _calc_pos:0.0,
            _is_moving:false,
            _drag_point:0.,
            _drag_pos_start:0.,
            _drag_max_pos:0.0,

            split_size:8.0,
            min_size:25.0,
            split:Quad{
                shader_id:cx.add_shader(split_sh,"Splitter.split"),
                ..Style::style(cx)
            },

            animator:Animator::new(Anim::new(AnimMode::Cut{duration:0.5},vec![
                AnimTrack::to_vec4("split.color",cx.style_color("bg_normal")),
            ])),
            anim_over:Anim::new(AnimMode::Cut{duration:0.05}, vec![
                AnimTrack::to_vec4("split.color", color("#5")),
            ]),
            anim_moving:Anim::new(AnimMode::Cut{duration:0.2}, vec![
                AnimTrack::vec4("split.color", Ease::Linear, vec![
                    (0.0, color("#f")),
                    (1.0, color("#6"))
                ]),
            ]),
        }
    }
}

impl Splitter{

    pub fn def_split_shader(cx:&mut Cx)->Shader{
        let mut sh = Quad::def_quad_shader(cx);
        sh.add_ast(shader_ast!({

            const border_radius:float = 1.5;

            fn pixel()->vec4{
                df_viewport(pos * vec2(w, h));
                df_box(0., 0., w, h, 0.5);
                return df_fill(color);
            }
        }));
        sh
    }

    pub fn handle_splitter(&mut self, cx:&mut Cx, event:&mut Event)->SplitterEvent{
        match event.hits(cx, self._split_area, &mut self._hit_state){
            Event::Animate(ae)=>{
                self.animator.calc_area(cx, "split.color", ae.time, self._split_area);
            },
            Event::FingerDown(fe)=>{
                self._is_moving = true;
                self.animator.play_anim(cx, self.anim_moving.clone());
                match self.axis{
                    Axis::Horizontal=>cx.set_down_mouse_cursor(MouseCursor::RowResize),
                    Axis::Vertical=>cx.set_down_mouse_cursor(MouseCursor::ColResize)
                };
                self._drag_pos_start = self.pos;
                self._drag_point = match self.axis{
                    Axis::Horizontal=>{fe.rel_y},
                    Axis::Vertical=>{fe.rel_x}
                }
            },
            Event::FingerHover(fe)=>{
                match self.axis{
                    Axis::Horizontal=>cx.set_hover_mouse_cursor(MouseCursor::RowResize),
                    Axis::Vertical=>cx.set_hover_mouse_cursor(MouseCursor::ColResize)
                };
                if !self._is_moving{
                    match fe.hover_state{
                        HoverState::In=>{
                            self.animator.play_anim(cx, self.anim_over.clone());
                        },
                        HoverState::Out=>{
                            self.animator.play_anim(cx, self.animator.default.clone());
                        },
                        _=>()
                    }
                }
            },
            Event::FingerUp(fe)=>{
                self._is_moving = false;
                if fe.is_over{
                    if !fe.is_touch{
                        self.animator.play_anim(cx, self.anim_over.clone());
                    }
                    else{
                        self.animator.play_anim(cx, self.animator.default.clone());
                    }
                }
                else{
                    self.animator.play_anim(cx, self.animator.default.clone());
                }
            },
            Event::FingerMove(fe)=>{

                let delta = match self.axis{
                    Axis::Horizontal=>{
                        fe.abs_start_y - fe.abs_y
                    },
                    Axis::Vertical=>{
                        fe.abs_start_x - fe.abs_x
                    }
                };
                let mut pos = match self.align{
                    SplitterAlign::First=>self._drag_pos_start - delta,
                    SplitterAlign::Last=>self._drag_pos_start + delta,
                    SplitterAlign::Weighted=>self._drag_pos_start * self._drag_max_pos - delta
                };
                if pos > self._drag_max_pos - self.min_size{
                    pos = self._drag_max_pos - self.min_size
                }
                else if pos < self.min_size{
                    pos = self.min_size
                };
                let calc_pos = match self.align{
                    SplitterAlign::First=>{
                        self.pos = pos;
                        pos
                    },
                    SplitterAlign::Last=>{
                        self.pos = pos;
                        self._drag_max_pos - pos
                    },
                    SplitterAlign::Weighted=>{
                        self.pos = pos / self._drag_max_pos;
                        pos
                    }
                };
                if calc_pos != self._calc_pos{
                    self._calc_pos = calc_pos;
                    cx.redraw_area(self._split_area);
                    return SplitterEvent::Moving{new_pos:self.pos};
                }
            }
            _=>()
        };
        SplitterEvent::None
    }

    pub fn set_splitter_state(&mut self, align:SplitterAlign, pos:f32, axis:Axis){
       self.axis = axis;
       self.align = align;
       self.pos = pos;
    }

    pub fn begin_splitter(&mut self, cx:&mut Cx){
       let rect = cx.turtle_rect();
       self._calc_pos = match self.align{
           SplitterAlign::First=>self.pos,
           SplitterAlign::Last=>match self.axis{
               Axis::Horizontal=>rect.h - self.pos,
               Axis::Vertical=>rect.w - self.pos
           },
           SplitterAlign::Weighted=>self.pos * match self.axis{
               Axis::Horizontal=>rect.h,
               Axis::Vertical=>rect.w 
           }
       };
       match self.axis{
            Axis::Horizontal=>{
                cx.begin_turtle(&Layout{
                    width:Bounds::Fill,
                    height:Bounds::Fix(self._calc_pos),
                    ..Default::default()
                }, Area::Empty)
            },
            Axis::Vertical=>{
                cx.begin_turtle(&Layout{
                    width:Bounds::Fix(self._calc_pos),
                    height:Bounds::Fill,
                    ..Default::default()
                }, Area::Empty)
            }
       }
   }

    pub fn mid_splitter(&mut self, cx:&mut Cx){
        cx.end_turtle(Area::Empty);
        match self.axis{
            Axis::Horizontal=>{
                cx.move_turtle(0.0,self._calc_pos + self.split_size);
            },
            Axis::Vertical=>{
                cx.move_turtle(self._calc_pos + self.split_size, 0.0);
            }
       };
       cx.begin_turtle(&Layout{..Default::default()},Area::Empty);
   }

    pub fn end_splitter(&mut self, cx:&mut Cx){
        cx.end_turtle(Area::Empty);
        // draw the splitter in the middle of the turtle
        let rect = cx.turtle_rect();
        self.split.color = self.animator.last_vec4("split.color");
        match self.axis{
            Axis::Horizontal=>{
                self._split_area = self.split.draw_quad(cx, 0., self._calc_pos, rect.w, self.split_size);
                self._drag_max_pos = rect.h;
            },
            Axis::Vertical=>{
                self._split_area = self.split.draw_quad(cx, self._calc_pos, 0., self.split_size, rect.h);
                self._drag_max_pos = rect.w;
            }
       };
       self.animator.set_area(cx, self._split_area);
    }
}
