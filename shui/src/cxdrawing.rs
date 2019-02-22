use crate::shader::*;
use crate::cx::*;
use crate::cxshaders::*;
pub use crate::cxturtle::*;
pub use crate::cxturtle::Value::Computed;
pub use crate::cxturtle::Value::Fixed;
pub use crate::cxturtle::Value::Percent;
pub use crate::cxturtle::Value::Expression;

pub enum Ev{
    Redraw,
    Animate,
    FingerMove{x:f32, y:f32},
    FingerDown{x:f32, y:f32},
    FingerUp{x:f32, y:f32},
}

#[derive(Clone, Default)]
pub struct InstanceRef{
    pub draw_list_id:usize,
    pub draw_id:usize,
    pub instance_offset:usize,
}

#[derive(Clone, Default)]
pub struct CxDrawing{
    pub draw_lists: Vec<DrawList>,
    pub draw_lists_free: Vec<usize>,
    pub instance_nesting: Vec<InstanceRef>,
    pub view_stack: Vec<View>,
    pub draw_list_id: usize,
    pub frame_id: usize
}

impl CxDrawing{
    pub fn draw_list(&mut self)->&mut DrawList{
        &mut self.draw_lists[self.draw_list_id]
    }

    pub fn instance_aligned(&mut self, sh:&CompiledShader, turtle:&mut CxTurtle)->&mut Draw{
        let draw_list_id = self.draw_list_id;
        let dc = self.instance(sh);
        turtle.align_list.push(AlignItem{
            draw_list_id:draw_list_id,
            draw_id:dc.draw_id,
            instance_offset:dc.current_instance_offset,
            instance_count:1
        });
        dc
    }

    pub fn instance(&mut self, sh:&CompiledShader)->&mut Draw{
        let draw_list = &mut self.draw_lists[self.draw_list_id];
        
        // find our drawcall in the filled draws
        for i in (0..draw_list.draws_len).rev(){
            if draw_list.draws[i].shader_id == sh.shader_id{
                // reuse this drawcmd.
                let dc = &mut draw_list.draws[i];
                dc.current_instance_offset = dc.instance.len();
                dc.first = false;
                return dc
            }
        }  

        // we need a new draw
        let id = draw_list.draws_len;
        draw_list.draws_len = draw_list.draws_len + 1;
        
        // see if we need to add a new one
        if id >= draw_list.draws.len(){
            draw_list.draws.push(Draw{
                draw_id:draw_list.draws.len(),
                sub_list_id:0,
                shader_id:sh.shader_id,
                instance:Vec::new(),
                uniforms:Vec::new(),
                textures:Vec::new(),
                current_instance_offset:0,
                first:true,
                update_frame_id:self.frame_id,
                vao:CxShaders::create_vao(sh),
                buffers:DrawBuffers{..Default::default()}
            });
            return &mut draw_list.draws[id];
        }

        // reuse a draw
        let draw = &mut draw_list.draws[id];
        // we used to be a sublist, construct vao
        if draw.sub_list_id != 0{
            draw.shader_id = sh.shader_id;
            draw.vao = CxShaders::create_vao(sh);
        }
        // used to be another shader, destroy/construct vao
        else if draw.shader_id != sh.shader_id{
            CxShaders::destroy_vao(&mut draw.vao);
            draw.vao = CxShaders::create_vao(sh);
            draw.shader_id = sh.shader_id;
        }
        // truncate buffers and set update frame
        draw.instance.truncate(0);
        draw.current_instance_offset = 0;
        draw.uniforms.truncate(0);
        draw.textures.truncate(0);
        draw.update_frame_id = self.frame_id;
        draw.first = true;
        draw
    }

    // push instance so it can be written to again in pop_instance
    pub fn push_instance(&mut self)->&mut Draw{
        let draw_list = &mut self.draw_lists[self.draw_list_id];
        let draw = &mut draw_list.draws[draw_list.draws_len - 1];

        // store our current instance properties so we can update-patch it in pop instance
        self.instance_nesting.push(InstanceRef{
            draw_list_id: self.draw_list_id,
            draw_id:draw_list.draws_len - 1,
            instance_offset:draw.current_instance_offset
        });
        draw
    }

    // pops instance patching the supplied geometry in the instancebuffer
    pub fn pop_instance(&mut self, cxsh:&CxShaders, geom:Rect){
        let ir = self.instance_nesting.pop().unwrap();
        let draw_list = &mut self.draw_lists[ir.draw_list_id];
        let draw = &mut draw_list.draws[ir.draw_id];

        // ok now we have to patch x/y/w/h into it
        let csh = &cxsh.compiled_shaders[draw.shader_id];
        if let Some(x) = csh.named_instance_props.x{
            draw.instance[ir.instance_offset + x] = geom.x;
        }
        if let Some(y) = csh.named_instance_props.y{
            draw.instance[ir.instance_offset + y] = geom.y;
        }
        if let Some(w) = csh.named_instance_props.w{
            draw.instance[ir.instance_offset + w] = geom.w;
        }
        if let Some(h) = csh.named_instance_props.h{
            draw.instance[ir.instance_offset + h] = geom.h;
        }
    }
}

#[derive(Default,Clone)]
pub struct GLInstanceVAO{
    pub vao:gl::types::GLuint,
    pub vb:gl::types::GLuint
}

#[derive(Default,Clone)]
pub struct Draw{
    pub draw_id:usize,
    pub sub_list_id:usize, // if not 0, its a subnode
    pub shader_id:usize, // if shader_id changed, delete gl vao
    pub instance:Vec<f32>,
    pub current_instance_offset:usize, // offset of current instance
    pub uniforms:Vec<f32>,  // draw uniforms
    pub textures:Vec<usize>,
    pub update_frame_id: usize,
    pub vao:GLInstanceVAO,
    pub buffers:DrawBuffers,
    pub first:bool
}

impl Draw{
    pub fn float(&mut self, _name: &str, v:f32){
        self.instance.push(v);
    }

    pub fn rect(&mut self, _name: &str, rect:Rect){
        self.instance.push(rect.x);
        self.instance.push(rect.y);
        self.instance.push(rect.w);
        self.instance.push(rect.h);
    }

    pub fn vec2f(&mut self, _name: &str, x:f32, y:f32){
        self.instance.push(x);
        self.instance.push(y);
    }

    pub fn vec3f(&mut self, _name: &str, x:f32, y:f32, z:f32){
        self.instance.push(x);
        self.instance.push(y);
        self.instance.push(z);
    }

    pub fn vec4f(&mut self, _name: &str, x:f32, y:f32, z:f32, w:f32){
        self.instance.push(x);
        self.instance.push(y);
        self.instance.push(z);
        self.instance.push(w);
    }

    pub fn vec2(&mut self, _name: &str, v:&Vec2){
        self.instance.push(v.x);
        self.instance.push(v.y);
    }

    pub fn vec3(&mut self, _name: &str, v:&Vec3){
        self.instance.push(v.x);
        self.instance.push(v.y);
        self.instance.push(v.z);
    }

    pub fn vec4(&mut self, _name: &str, v:&Vec4){
        self.instance.push(v.x);
        self.instance.push(v.y);
        self.instance.push(v.z);
        self.instance.push(v.w);
    }

    pub fn texture(&mut self, _name: &str, texture_id: usize){
        // how do we store these?
        self.textures.push(texture_id);
    }

    pub fn ufloat(&mut self, _name: &str, v:f32){
        self.uniforms.push(v);
    }

    pub fn uvec2f(&mut self, _name: &str, x:f32, y:f32){
        self.uniforms.push(x);
        self.uniforms.push(y);
    }

    pub fn uvec3f(&mut self, _name: &str, x:f32, y:f32, z:f32){
        self.uniforms.push(x);
        self.uniforms.push(y);
        self.uniforms.push(z);
    }

    pub fn uvec4f(&mut self, _name: &str, x:f32, y:f32, z:f32, w:f32){
        self.uniforms.push(x);
        self.uniforms.push(y);
        self.uniforms.push(z);
        self.uniforms.push(w);
    }

    pub fn uvec2(&mut self, _name: &str, v:&Vec2){
        self.uniforms.push(v.x);
        self.uniforms.push(v.y);
    }

    pub fn uvec3(&mut self, _name: &str, v:&Vec3){
        self.uniforms.push(v.x);
        self.uniforms.push(v.y);
        self.uniforms.push(v.z);
    }

    pub fn uvec4(&mut self, _name: &str, v:&Vec4){
        self.uniforms.push(v.x);
        self.uniforms.push(v.y);
        self.uniforms.push(v.z);
        self.uniforms.push(v.w);
    }

    pub fn umat4(&mut self, _name: &str, v:&Mat4){
        for i in 0..16{
            self.uniforms.push(v.v[i]);
        }
    }
}

// CX and DL uniforms
const DL_UNI_PROP2:usize = 0;
const DL_UNI_SIZE:usize = 1;

#[derive(Default,Clone)]
pub struct DrawList{
    pub draws:Vec<Draw>,
    pub draws_len: usize,
    pub uniforms:Vec<f32>, // cmdlist uniforms
    pub buffers:DrawListBuffers
}

impl DrawList{
    pub fn initialize(&mut self){
        self.uniforms.resize(DL_UNI_SIZE, 0.0);
    }
    
    pub fn def_uniforms(_sh:&mut Shader){
        //sh.dl_uniform("prop2", Kind::Float);
    }

    pub fn uniform_prop2(&mut self, v:f32){
        self.uniforms[DL_UNI_PROP2] = v;
    }
}

pub trait Style{
    fn style(cx:&mut Cx) -> Self;
}

#[derive(Default,Clone)]
pub struct View{ // draw info per UI element
    pub id:usize,
    pub frame_id:usize,
    pub initialized:bool,
    // the set of shader_id + 
    pub draw_list_id:usize 
}

impl View{
    pub fn new()->Self{
        Self{
            ..Default::default()
        }
    }

    pub fn begin(&mut self, cx:&mut Cx, layout:&Layout){
        if !self.initialized{ // draw node needs initialization
            if cx.drawing.draw_lists_free.len() != 0{
                self.draw_list_id = cx.drawing.draw_lists_free.pop().unwrap();
            }
            else{
                self.draw_list_id = cx.drawing.draw_lists.len();
                cx.drawing.draw_lists.push(DrawList{..Default::default()});
            }
            self.initialized = true;
            let draw_list = &mut cx.drawing.draw_lists[self.draw_list_id];
            draw_list.initialize();
        }
        else{
            // set len to 0
            let draw_list = &mut cx.drawing.draw_lists[self.draw_list_id];
            draw_list.draws_len = 0;
        }
        // push ourselves up the parent draw_stack
        if let Some(parent_view) = cx.drawing.view_stack.last(){

            // we need a new draw
            let parent_draw_list = &mut cx.drawing.draw_lists[parent_view.draw_list_id];

            let id = parent_draw_list.draws_len;
            parent_draw_list.draws_len = parent_draw_list.draws_len + 1;
            
            // see if we need to add a new one
            if parent_draw_list.draws_len > parent_draw_list.draws.len(){
                parent_draw_list.draws.push({
                    Draw{
                        sub_list_id:self.draw_list_id,
                        ..Default::default()
                    }
                })
            }
            else{// or reuse a sub list node
                let draw = &mut parent_draw_list.draws[id];
                if draw.sub_list_id == 0{ // we used to be a drawcmd
                    CxShaders::destroy_vao(&mut draw.vao);
                    draw.sub_list_id = self.draw_list_id;
                }
                else{ // used to be a sublist
                    draw.sub_list_id = self.draw_list_id;
                }
            }
        }

        cx.drawing.draw_list_id = self.draw_list_id;
        cx.drawing.view_stack.push(self.clone());
        
        cx.turtle.begin(layout);
        //cx.turtle.x = 0.0;
        //cx.turtle.y = 0.0;
    }

    pub fn end(&mut self, cx:&mut Cx){
        cx.drawing.view_stack.pop();
        cx.turtle.end(&mut cx.drawing,&cx.shaders);
    }
}