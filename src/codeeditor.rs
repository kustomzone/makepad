use widgets::*;
use crate::textbuffer::*;

#[derive(Clone)]
pub struct CodeEditor{
    pub view:View<ScrollBar>,
    pub bg_layout:Layout,
    pub bg: Quad,
    pub cursor: Quad,
    pub marker: Quad,
    pub tab:Quad,
    pub text: Text,
    pub cursors:CursorSet,
    pub _hit_state:HitState,
    pub _bg_area:Area,
    pub _text_inst:Option<AlignedInstance>,
    pub _text_area:Area,
    pub _scroll_pos:Vec2,
    pub _last_finger_move:Option<Vec2>,
    pub _line_geometry:Vec<LineGeom>,
    pub _token_chunks:Vec<TokenChunk>,
    pub _visible_lines:usize,
    pub _visibility_margin:Margin,
    pub _select_scroll:Option<SelectScroll>,
    pub _grid_select_corner:Option<TextPos>,

    pub _monospace_size:Vec2,
    pub _instance_count:usize,
    pub _first_on_line:bool,
    pub _draw_cursor:DrawCursor
}

#[derive(Clone, Default)]
pub struct LineGeom{
    walk:Vec2,
    font_size:f32
}

#[derive(Clone, Default)]
pub struct SelectScroll{
    pub margin:Margin,
    pub delta:Vec2,
    pub abs:Vec2
}

impl ElementLife for CodeEditor{
    fn construct(&mut self, _cx:&mut Cx){}
    fn destruct(&mut self, _cx:&mut Cx){}
}

impl Style for CodeEditor{
    fn style(cx:&mut Cx)->Self{
        let tab_sh = Self::def_tab_shader(cx);
        let marker_sh = Self::def_marker_shader(cx);
        let cursor_sh = Self::def_cursor_shader(cx);
        let code_editor = Self{
            cursors:CursorSet::new(),
            tab:Quad{
                color:color("#5"),
                shader_id:cx.add_shader(tab_sh, "Editor.tab"),
                ..Style::style(cx)
            },
            view:View{
                scroll_h:Some(ScrollBar{
                    ..Style::style(cx)
                }),
                scroll_v:Some(ScrollBar{
                    smoothing:Some(0.15),
                    ..Style::style(cx)
                }),
                ..Style::style(cx)
            },
            bg:Quad{
                color:color256(30,30,30),
                do_scroll:false,
                ..Style::style(cx)
            },
            marker:Quad{
                color:color256(42,78,117),
                shader_id:cx.add_shader(marker_sh, "Editor.marker"),
                ..Style::style(cx)
            }, 
            cursor:Quad{
                color:color256(136,136,136),
                shader_id:cx.add_shader(cursor_sh, "Editor.cursor"),
                ..Style::style(cx)
            },
            bg_layout:Layout{
                width:Bounds::Fill,
                height:Bounds::Fill,
                margin:Margin::all(0.),
                padding:Padding{l:4.0,t:4.0,r:4.0,b:4.0},
                ..Default::default()
            },
            text:Text{
                font_id:cx.load_font(&cx.font("mono_font")),
                font_size:11.0,
                brightness:1.05,
                line_spacing:1.4,
                wrapping:Wrapping::Line,
                ..Style::style(cx)
            },
            _hit_state:HitState{no_scrolling:true, ..Default::default()},
            _monospace_size:Vec2::zero(),
            _last_finger_move:None,
            _first_on_line:true,
            _scroll_pos:Vec2::zero(),
            _visibility_margin:Margin::zero(),
            _visible_lines:0, 
            _line_geometry:Vec::new(),
            _token_chunks:Vec::new(),
            _grid_select_corner:None,
            _bg_area:Area::Empty,
            _text_inst:None,
            _text_area:Area::Empty,
            _instance_count:0,
            _select_scroll:None,
            _draw_cursor:DrawCursor::new()
        };
        //tab.animator.default = tab.anim_default(cx);
        code_editor
    }
}

#[derive(Clone, PartialEq)]
pub enum CodeEditorEvent{
    None,
    Change
}

impl CodeEditor{

    pub fn def_tab_shader(cx:&mut Cx)->Shader{
        let mut sh = Quad::def_quad_shader(cx);
        sh.add_ast(shader_ast!({
            fn pixel()->vec4{
                df_viewport(pos * vec2(w, h));
                df_move_to(1.,-1.);
                df_line_to(1.,h+1.);
                return df_stroke(color, 0.8);
            }
        }));
        sh
    }

    pub fn def_cursor_shader(cx:&mut Cx)->Shader{
        let mut sh = Quad::def_quad_shader(cx);
        sh.add_ast(shader_ast!({
            fn pixel()->vec4{
                return vec4(color.rgb*color.a,color.a)
            }
        }));
        sh
    }

    pub fn def_marker_shader(cx:&mut Cx)->Shader{
        let mut sh = Quad::def_quad_shader(cx);
        sh.add_ast(shader_ast!({
            let prev_x:float<Instance>;
            let prev_w:float<Instance>;
            let next_x:float<Instance>;
            let next_w:float<Instance>;
            const gloopiness:float = 8.;
            const border_radius:float = 2.;

            fn vertex()->vec4{ // custom vertex shader because we widen the draweable area a bit for the gloopiness
                let shift:vec2 = -draw_list_scroll * draw_list_do_scroll;
                let clipped:vec2 = clamp(
                    geom*vec2(w+16., h) + vec2(x, y) + shift - vec2(8.,0.),
                    draw_list_clip.xy,
                    draw_list_clip.zw
                );
                pos = (clipped - shift - vec2(x,y)) / vec2(w, h);
                return vec4(clipped,0.,1.) * camera_projection;
            }

            fn pixel()->vec4{
                df_viewport(pos * vec2(w, h));
                df_box(0., 0., w, h, border_radius);
                if prev_w > 0.{
                    df_box(prev_x, -h, prev_w, h, border_radius);
                    df_gloop(gloopiness);
                }
                if next_w > 0.{
                    df_box(next_x, h, next_w, h, border_radius);
                    df_gloop(gloopiness);
                }
                return df_fill(color);
            }
        }));
        sh
    }

    pub fn handle_code_editor(&mut self, cx:&mut Cx, event:&mut Event, text_buffer:&mut TextBuffer)->CodeEditorEvent{
        match self.view.handle_scroll_bars(cx, event){
            (_,ScrollBarEvent::Scroll{..}) | (ScrollBarEvent::Scroll{..},_)=>{
                if let Some(last_finger_move) = self._last_finger_move{
                    if let Some(grid_select_corner) = self._grid_select_corner{
                        let pos = self.compute_grid_text_pos_from_abs(cx, last_finger_move);
                        self.cursors.grid_select(grid_select_corner, pos, text_buffer);
                    }
                    else{
                        let offset = self.text.find_closest_offset(cx, &self._text_area, last_finger_move);
                        self.cursors.set_last_cursor_head(offset, text_buffer);
                    }
                }
                // the editor actually redraws on scroll, its because we don't actually
                // generate the entire file as GPU text-buffer just the visible area
                // in JS this wasn't possible performantly but in Rust its a breeze.
                self.view.redraw_view_area(cx);
            },
            _=>()
        }
        match event.hits(cx, self._bg_area, &mut self._hit_state){

            Event::Animate(_ae)=>{
            },
            Event::FingerDown(fe)=>{
                cx.set_down_mouse_cursor(MouseCursor::Text);
                // give us the focus
                cx.set_key_focus(self._bg_area);
                let offset = self.text.find_closest_offset(cx, &self._text_area, fe.abs);
                match fe.tap_count{
                    2=>{
                        let range = self.get_nearest_token_chunk_range(offset);
                        self.cursors.set_last_clamp_range(range);
                    },
                    3=>{
                        let range = text_buffer.get_nearest_line_range(offset);
                        self.cursors.set_last_clamp_range(range);
                    },
                    4=>{
                        let range = (0, text_buffer.get_char_count());
                        self.cursors.set_last_clamp_range(range);
                    },
                    _=>()
                }
                // ok so we should scan a range 

                if fe.modifiers.shift{
                    if !fe.modifiers.logo{ // simply place selection
                        self.cursors.clear_and_set_last_cursor_head(offset, text_buffer);
                    }
                    else{ // grid select
                        // we need to figure out wether we'll pick 
                        // essentially what we do is on mousemove just 'create' all cursors
                        let pos = self.compute_grid_text_pos_from_abs(cx, fe.abs);
                        self._grid_select_corner = Some(self.cursors.grid_select_corner(pos, text_buffer));
                        self.cursors.grid_select(self._grid_select_corner.unwrap(), pos, text_buffer);
                        //self._is_grid_select = true;
                    }
                }
                else{ // cursor drag with possible add
                    if fe.modifiers.logo{
                        self.cursors.add_last_cursor_head_and_tail(offset, text_buffer);
                    }
                    else{
                        self.cursors.clear_and_set_last_cursor_head_and_tail(offset, text_buffer);
                    }
                }
                self.view.redraw_view_area(cx);
                self._last_finger_move = Some(fe.abs);
            },
            Event::FingerHover(_fe)=>{
                cx.set_hover_mouse_cursor(MouseCursor::Text);
            },
            Event::FingerUp(_fe)=>{
                self.cursors.clear_last_clamp_range();
                //self.cursors.end_cursor_drag(text_buffer);
                self._select_scroll = None;
                self._last_finger_move = None;
                self._grid_select_corner = None;
            },
            Event::FingerMove(fe)=>{
                if let Some(grid_select_corner) = self._grid_select_corner{
                    let pos = self.compute_grid_text_pos_from_abs(cx, fe.abs);
                    self.cursors.grid_select(grid_select_corner, pos, text_buffer);
                }
                else{
                    let offset = self.text.find_closest_offset(cx, &self._text_area, fe.abs);
                    self.cursors.set_last_cursor_head(offset, text_buffer);
                }

                self._last_finger_move = Some(fe.abs);
                // determine selection drag scroll dynamics
                let pow_scale = 0.1;
                let pow_fac = 3.;
                let max_speed = 40.;
                let pad_scroll = 20.;
                let rect = Rect{
                    x:fe.rect.x+pad_scroll,
                    y:fe.rect.y+pad_scroll,
                    w:fe.rect.w-2.*pad_scroll,
                    h:fe.rect.h-2.*pad_scroll,
                };
                let delta = Vec2{
                    x:if fe.abs.x < rect.x{
                        -((rect.x - fe.abs.x) * pow_scale).powf(pow_fac).min(max_speed)
                    }
                    else if fe.abs.x > rect.x + rect.w{
                        ((fe.abs.x - (rect.x + rect.w)) * pow_scale).powf(pow_fac).min(max_speed)
                    }
                    else{
                        0.
                    },
                    y:if fe.abs.y < rect.y{
                        -((rect.y - fe.abs.y) * pow_scale).powf(pow_fac).min(max_speed)
                    }
                    else if fe.abs.y > rect.y + rect.h{
                        ((fe.abs.y - (rect.y + rect.h)) * pow_scale).powf(pow_fac).min(max_speed)
                    }
                    else{
                        0.
                    }
                };
                let last_scroll_none = self._select_scroll.is_none();
                if delta.x !=0. || delta.y != 0.{
                   self._select_scroll = Some(SelectScroll{
                       abs:fe.abs,
                       delta:delta,
                       margin:Margin{
                            l:(-delta.x).max(0.),
                            t:(-delta.y).max(0.),
                            r:delta.x.max(0.),
                            b:delta.y.max(0.)
                        }
                   })
                }
                else{
                    self._select_scroll = None;
                }
                if last_scroll_none{
                    self.view.redraw_view_area(cx);
                }
            },
            Event::KeyDown(ke)=>{
                let cursor_moved = match ke.key_code{
                    KeyCode::ArrowUp=>{
                        self.cursors.move_up(1, ke.modifiers.shift, text_buffer);
                        true
                    },
                    KeyCode::ArrowDown=>{
                        self.cursors.move_down(1, ke.modifiers.shift, text_buffer);
                        true
                    },
                    KeyCode::ArrowLeft=>{
                        if ke.modifiers.logo{ // token skipping
                            self.cursors.move_left_nearest_token(ke.modifiers.shift, &self._token_chunks, text_buffer)
                        }
                        else{
                            self.cursors.move_left(1, ke.modifiers.shift, text_buffer);
                        }
                        true
                    },
                    KeyCode::ArrowRight=>{
                        if ke.modifiers.logo{ // token skipping
                            self.cursors.move_right_nearest_token(ke.modifiers.shift, &self._token_chunks, text_buffer)
                        }
                        else{
                            self.cursors.move_right(1, ke.modifiers.shift, text_buffer);
                        }
                        true
                    },
                    KeyCode::PageUp=>{
                        
                        self.cursors.move_up(self._visible_lines.max(5) - 4, ke.modifiers.shift, text_buffer);
                        true
                    },
                    KeyCode::PageDown=>{
                        self.cursors.move_down(self._visible_lines.max(5) - 4, ke.modifiers.shift, text_buffer);
                        true
                    },
                    KeyCode::Home=>{
                        self.cursors.move_home(ke.modifiers.shift, text_buffer);
                        true
                    },
                    KeyCode::End=>{
                        self.cursors.move_end(ke.modifiers.shift, text_buffer);
                        true
                    },
                    KeyCode::Backspace=>{
                        self.cursors.backspace(text_buffer);
                        true
                    },
                    KeyCode::Delete=>{
                        self.cursors.delete(text_buffer);
                        true
                    },
                    KeyCode::KeyZ=>{
                        if ke.modifiers.logo || ke.modifiers.control{
                            if ke.modifiers.shift{ // redo
                                text_buffer.redo(true, &mut self.cursors);
                                true
                            }
                            else{ // undo
                                text_buffer.undo(true, &mut self.cursors);
                                true
                            }
                        }
                        else{
                            false
                        }
                    },
                    KeyCode::KeyX=>{ // cut
                        if ke.modifiers.logo || ke.modifiers.control{ // cut
                            self.cursors.replace_text("", text_buffer);
                            true
                        }
                        else{
                            false
                        }
                    },
                    KeyCode::KeyA=>{ // select all
                        if ke.modifiers.logo || ke.modifiers.control{ // cut
                            self.cursors.select_all(text_buffer);
                            // don't scroll!
                            self.view.redraw_view_area(cx);
                            false
                        }
                        else{
                            false
                        }
                    }
                    _=>false
                };
                if cursor_moved{
                    self.scroll_last_cursor_visible(cx, text_buffer);
                    self.view.redraw_view_area(cx);
                }
            },
            Event::TextInput(te)=>{
                if te.replace_last{
                    text_buffer.undo(false, &mut self.cursors);
                }
                self.cursors.replace_text(&te.input, text_buffer);
                self.scroll_last_cursor_visible(cx, text_buffer);
                self.view.redraw_view_area(cx);
            },
            Event::TextCopy(_)=>match event{ // access the original event
                Event::TextCopy(req)=>{
                    req.response = Some(self.cursors.get_all_as_string(text_buffer));
                },
                _=>()
            },
            _=>()
        };
        CodeEditorEvent::None
   }

    pub fn begin_code_editor(&mut self, cx:&mut Cx, text_buffer:&TextBuffer)->bool{
        // pull the bg color from our animation system, uses 'default' value otherwise
        // self.bg.color = self.animator.last_vec4("bg.color");
        // push the 2 vars we added to bg shader
        //self.text.color = self.animator.last_vec4("text.color");
        self.view.begin_view(cx, &Layout{..Default::default()});
        //   return false
        //}
        if text_buffer.load_id != 0{
            let bg_inst = self.bg.begin_quad(cx, &Layout{
                align:Align::left_top(),
                ..self.bg_layout.clone()
            });
            self.text.color = color("#666");
            self.text.draw_text(cx, "...");
            self.bg.end_quad(cx, &bg_inst);
            self._bg_area = bg_inst.into_area();
            self.view.end_view(cx);
            return false
        }
        else{

            let bg_inst = self.bg.draw_quad(cx, Rect{x:0.,y:0., w:cx.width_total(false), h:cx.height_total(false)});
            let bg_area = bg_inst.into_area();
            cx.update_area_refs(self._bg_area, bg_area);
            self._bg_area = bg_area;
            // makers before text
            cx.new_instance_layer(self.marker.shader_id, 0);

            self._text_inst = Some(self.text.begin_text(cx));
            self._instance_count = 0;

            self._scroll_pos = self.view.get_scroll_pos(cx);

            self._visibility_margin = if let Some(select_scroll) = &self._select_scroll{
                select_scroll.margin
            }
            else{
                Margin::zero()
            };

            self._monospace_size = self.text.get_monospace_size(cx, None);
            self._line_geometry.truncate(0);
            self._token_chunks.truncate(0);
            self._draw_cursor = DrawCursor::new();
            self._first_on_line = true;
            self._visible_lines = 0;
            // prime the next cursor
            self._draw_cursor.set_next(&self.cursors.set);
            // cursor after text
            cx.new_instance_layer(self.cursor.shader_id, 0);
            
            return true
        }
    }
    
    pub fn end_code_editor(&mut self, cx:&mut Cx, text_buffer:&TextBuffer){
        // lets insert an empty newline at the bottom so its nicer to scroll
        cx.turtle_new_line();
        cx.walk_turtle(Bounds::Fix(0.0),  Bounds::Fix(self._monospace_size.y),  Margin::zero(), None);
        
        self.text.end_text(cx, self._text_inst.as_ref().unwrap());
        // lets draw cursors and selection rects.
        //let draw_cursor = &self._draw_cursor;
        let pos = cx.turtle_origin();
        cx.new_instance_layer(self.cursor.shader_id, 0);

        // draw the cursors    
        for rc in &self._draw_cursor.cursors{
           self.cursor.draw_quad(cx, Rect{x:rc.x - pos.x, y:rc.y - pos.y, w:rc.w, h:rc.h});
        }

        
        self._text_area = self._text_inst.take().unwrap().inst.into_area();

        // draw selections
        let sel = &self._draw_cursor.selections;
        for i in 0..sel.len(){
            let cur = &sel[i];
            let mk_inst = self.marker.draw_quad(cx, Rect{x:cur.rc.x - pos.x, y:cur.rc.y - pos.y, w:cur.rc.w, h:cur.rc.h});
            // do we have a prev?
            if i > 0 && sel[i-1].index == cur.index{
                let p_rc = &sel[i-1].rc;
                mk_inst.push_vec2(cx, Vec2{x:p_rc.x - cur.rc.x, y:p_rc.w}); // prev_x, prev_w
            }
            else{
                mk_inst.push_vec2(cx, Vec2{x:0., y:-1.}); // prev_x, prev_w
            }
            // do we have a next
            if i < sel.len() - 1 && sel[i+1].index == cur.index{
                let n_rc = &sel[i+1].rc;
                mk_inst.push_vec2(cx, Vec2{x:n_rc.x - cur.rc.x, y:n_rc.w}); // prev_x, prev_w
            }
            else{
                mk_inst.push_vec2(cx, Vec2{x:0., y:-1.}); // prev_x, prev_w
            }
        }

        // do select scrolling
        if let Some(select_scroll) = self._select_scroll.clone(){
            if let Some(grid_select_corner) = self._grid_select_corner{
               // self.cursors.grid_select(offset, text_buffer);
                let pos = self.compute_grid_text_pos_from_abs(cx, select_scroll.abs);
                self.cursors.grid_select(grid_select_corner, pos, text_buffer);
            }
            else{
                let offset = self.text.find_closest_offset(cx, &self._text_area, select_scroll.abs);
                self.cursors.set_last_cursor_head(offset, text_buffer);
            }

            if self.view.set_scroll_pos(cx, Vec2{
                x:self._scroll_pos.x + select_scroll.delta.x,
                y:self._scroll_pos.y + select_scroll.delta.y
            }){
                self.view.redraw_view_area(cx);
            }
            else{
                self._select_scroll = None;
            }
        }

        self.view.end_view(cx);

        // place the IME
        if self._bg_area == cx.key_focus{
            if let Some(last_cursor) = self._draw_cursor.last_cursor{
                let rc = self._draw_cursor.cursors[last_cursor];
                let scroll_pos = self.view.get_scroll_pos(cx);
                cx.show_text_ime(rc.x - scroll_pos.x, rc.y - scroll_pos.y);
            }
            else{ // current last cursors is not visible
                cx.hide_text_ime();
            }
        }
    }

    pub fn draw_tab_lines(&mut self, cx:&mut Cx, tabs:usize){
        let walk = cx.get_turtle_walk();
        let tab_width = self._monospace_size.x*4.;
        if cx.visible_in_turtle(
            Rect{x:walk.x, y:walk.y, w:tab_width * tabs as f32, h:self._monospace_size.y}, 
            self._visibility_margin, 
            self._scroll_pos,
        ){
            for _i in 0..tabs{
                self.tab.draw_quad_walk(cx, Bounds::Fix(tab_width), Bounds::Fix(self._monospace_size.y), Margin::zero());
            }   
            cx.set_turtle_walk(walk);
        }
    }

    // set it once per line otherwise the LineGeom stuff isn't really working out.
    pub fn set_font_size(&mut self, cx:&Cx, font_size:f32){
        self.text.font_size = font_size;
        self._monospace_size = self.text.get_monospace_size(cx, None);
    }

    pub fn new_line(&mut self, cx:&mut Cx){
        // line geometry is used for scrolling look up of cursors
        self._line_geometry.push(
            LineGeom{
                walk:cx.get_rel_turtle_walk(),
                font_size:self.text.font_size
            }
        );
        // add a bit of room to the right
        cx.walk_turtle(
            Bounds::Fix(self._monospace_size.x * 3.), 
            Bounds::Fix(self._monospace_size.y), 
            Margin::zero(),
            None
        );
        cx.turtle_new_line();
        self._first_on_line = true;
        let mut draw_cursor = &mut self._draw_cursor;
        if !draw_cursor.first{ // we have some selection data to emit
           draw_cursor.emit_selection(true);
           draw_cursor.first = true;
        }
    }

    pub fn draw_text(&mut self, cx:&mut Cx, chunk:&Vec<char>, end_offset:usize, is_whitespace:bool, color:Color){
        if chunk.len()>0{
            
            self._token_chunks.push(TokenChunk{
                offset:end_offset - chunk.len() - 1,
                len:chunk.len(),
                is_whitespace:is_whitespace,
            });
            
            let geom = cx.walk_turtle(
                Bounds::Fix(self._monospace_size.x * (chunk.len() as f32)), 
                Bounds::Fix(self._monospace_size.y), 
                Margin::zero(),
                None
            );
            
            // lets check if the geom is visible
            if cx.visible_in_turtle(geom, self._visibility_margin, self._scroll_pos){

                if self._first_on_line{
                    self._first_on_line = false;
                    self._visible_lines += 1;
                }

                self.text.color = color;
                // we need to find the next cursor point we need to do something at
                let cursors = &self.cursors.set;
                let last_cursor = self.cursors.last_cursor;
                let draw_cursor = &mut self._draw_cursor;
                let height = self._monospace_size.y;

                self.text.add_text(cx, geom.x, geom.y, end_offset - chunk.len() - 1, self._text_inst.as_mut().unwrap(), &chunk, |unicode, offset, x, w|{
                    // check if we need to skip cursors
                    while offset >= draw_cursor.end{ // jump to next cursor
                        if offset == draw_cursor.end{ // process the last bit here
                             draw_cursor.process_geom(last_cursor, offset, x, geom.y, w, height);
                            draw_cursor.emit_selection(false);
                        }
                        if !draw_cursor.set_next(cursors){ // cant go further
                            return 0.0
                        }
                    }
                    // in current cursor range, update values
                    if offset >= draw_cursor.start && offset <= draw_cursor.end{
                        draw_cursor.process_geom(last_cursor, offset, x, geom.y, w, height);
                        if offset == draw_cursor.end{
                            draw_cursor.emit_selection(false);
                        }
                        if unicode == 10{
                            return 0.0
                        }
                        else if unicode == 32 && offset < draw_cursor.end{
                            return 2.0
                        }
                    }
                    return 0.0
                });
            }

            self._instance_count += chunk.len();
        }
    }

    fn scroll_last_cursor_visible(&mut self, cx:&mut Cx, text_buffer:&TextBuffer){
        // so we have to compute (approximately) the rect of our cursor
        if self.cursors.last_cursor >= self.cursors.set.len(){
            panic!("LAST CURSOR INVALID");
        }
        let offset = self.cursors.set[self.cursors.last_cursor].head;
        let pos = text_buffer.offset_to_text_pos(offset);
        // alright now lets query the line geometry
        if pos.row < self._line_geometry.len(){
            let geom = &self._line_geometry[pos.row];
            let mono_size = self.text.get_monospace_size(cx, Some(geom.font_size));
            let rect = Rect{
                x:(pos.col as f32) * mono_size.x,
                y:geom.walk.y - mono_size.y * 1.,
                w:mono_size.x * 4.,
                h:mono_size.y * 3.
            };
            // scroll this cursor into view
            self.view.scroll_into_view(cx, rect);
        }
    }

    fn compute_grid_text_pos_from_abs(&mut self, cx:&Cx, abs:Vec2)->TextPos{
        // 
        let rel = self._bg_area.abs_to_rel_scrolled(cx, abs);
        let mut mono_size = Vec2::zero();
        for (row, geom) in self._line_geometry.iter().enumerate(){
            //let geom = &self._line_geometry[pos.row];
            mono_size = self.text.get_monospace_size(cx, Some(geom.font_size));
            if rel.y < geom.walk.y || rel.y >= geom.walk.y && rel.y <= geom.walk.y + mono_size.y{ // its on the right line
                let col = (rel.x.max(0.) / mono_size.x) as usize; // do a dumb calc
                return TextPos{row:row, col:col};
            }
        }
        // otherwise the file is too short, lets use the last line
        TextPos{row:self._line_geometry.len() - 1, col: (rel.x.max(0.) / mono_size.x) as usize}
    }

    fn get_nearest_token_chunk_range(&self, offset:usize)->(usize, usize){
        let chunks = &self._token_chunks;
        for i in 0..chunks.len(){
            if chunks[i].is_whitespace{
                if offset == chunks[i].offset && i > 0{ // at the start of whitespace
                    return (chunks[i-1].offset, chunks[i-1].len)
                }
                else if offset == chunks[i].offset + chunks[i].len && i < chunks.len()-1{
                    return (chunks[i+1].offset, chunks[i+1].len)
                }
            };

            if offset >= chunks[i].offset && offset < chunks[i].offset + chunks[i].len{
                return (chunks[i].offset, chunks[i].len)
            }
        };
        (0,0)
    }

}


#[derive(Clone)]
pub struct DrawSel{
    index:usize,
    rc:Rect,
}

#[derive(Clone)]
pub struct DrawCursor{
    pub head:usize,
    pub start:usize,
    pub end:usize,
    pub next_index:usize,
    pub left_top:Vec2,
    pub right_bottom:Vec2,
    pub last_w:f32,
    pub first:bool,
    pub empty:bool,
    pub cursors:Vec<Rect>,
    pub last_cursor:Option<usize>,
    pub selections:Vec<DrawSel>
}

impl DrawCursor{
    pub fn new()->DrawCursor{
        DrawCursor{
            start:0,
            end:0,
            head:0,
            first:true,
            empty:true,
            next_index:0,
            left_top:Vec2::zero(),
            right_bottom:Vec2::zero(),
            last_w:0.0,
            cursors:Vec::new(),
            selections:Vec::new(),
            last_cursor:None
        }
    }

    pub fn set_next(&mut self, cursors:&Vec<Cursor>)->bool{
        if self.next_index < cursors.len(){
            self.emit_selection(false);
            let cursor = &cursors[self.next_index];
            let (start,end) = cursor.order();
            self.start = start;
            self.end = end;
            self.head = cursor.head;
            self.next_index += 1;
            self.last_w = 0.0;
            self.first = true;
            self.empty = true;
            true
        }
        else{
            false
        }
    }

    pub fn emit_cursor(&mut self, x:f32, y:f32, h:f32){
        self.cursors.push(Rect{
            x:x,
            y:y,
            w:1.5,
            h:h
        })
    }

    pub fn emit_selection(&mut self, on_new_line:bool){
        if !self.first{
            self.first = true;
            if !self.empty || on_new_line{
                self.selections.push(DrawSel{
                    index:self.next_index - 1,
                    rc:Rect{
                        x:self.left_top.x,
                        y:self.left_top.y,
                        w:(self.right_bottom.x - self.left_top.x) + if on_new_line{self.last_w} else {0.0},
                        h:self.right_bottom.y - self.left_top.y
                    }
                })
            }
        }
    }

    pub fn process_geom(&mut self, last_cursor:usize, offset:usize, x:f32, y:f32, w:f32, h:f32){
        if offset == self.head{ // emit a cursor
            if self.next_index - 1 == last_cursor{
                self.last_cursor = Some(self.cursors.len());
            }
            self.emit_cursor(x, y, h);
        }
        if self.first{ // store left top of rect
            self.first = false;
            self.left_top.x = x;
            self.left_top.y = y;
            self.empty = true;
        }
        else{
            self.empty = false;
        }
        // current right/bottom
        self.last_w = w;
        self.right_bottom.x = x;
        self.right_bottom.y = y + h;
    }
}