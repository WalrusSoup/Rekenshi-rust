//! Decorated exports used by the verified native adapters.
//!
//! Grouped by DLL and C++ class. These names identify exports; each adapter
//! still verifies the owning binary and supplies its inspected function ABI.
#![forbid(unsafe_code)]

/// Generated discovery catalog. Export names still require inspected ABI bindings.
pub mod discovered;

/// Exports from `MyGUIEngine_x64.dll`.
pub mod mygui {
    use std::ffi::CStr;

    pub mod any {
        use super::CStr;

        pub const COPY_CONSTRUCT: &CStr = c"??0Any@MyGUI@@QEAA@AEBV01@@Z";
        pub const DESTROY: &CStr = c"??1Any@MyGUI@@QEAA@XZ";
        pub const GET_TYPE: &CStr = c"?getType@Any@MyGUI@@QEBAAEBVtype_info@@XZ";
    }

    pub mod button {
        use super::CStr;

        pub const SET_STATE_SELECTED: &CStr = c"?setStateSelected@Button@MyGUI@@QEAAX_N@Z";
    }

    pub mod gui {
        use super::CStr;

        pub const FIND_WIDGET_T: &CStr = c"?findWidgetT@Gui@MyGUI@@QEAAPEAVWidget@2@AEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@_N@Z";
        pub const FRAME_EVENT: &CStr = c"?frameEvent@Gui@MyGUI@@QEAAXM@Z";
        pub const GET_ENUMERATOR: &CStr = c"?getEnumerator@Gui@MyGUI@@QEBA?AV?$Enumerator@V?$vector@PEAVWidget@MyGUI@@V?$allocator@PEAVWidget@MyGUI@@@std@@@std@@@2@XZ";
    }

    pub mod i_cropped_rectangle {
        use super::CStr;

        pub const GET_HEIGHT: &CStr = c"?getHeight@ICroppedRectangle@MyGUI@@QEBAHXZ";
        pub const GET_WIDTH: &CStr = c"?getWidth@ICroppedRectangle@MyGUI@@QEBAHXZ";
    }

    pub mod tab_control {
        use super::CStr;

        pub const ADD_ITEM: &CStr =
            c"?addItem@TabControl@MyGUI@@QEAAPEAVTabItem@2@AEBVUString@2@VAny@2@@Z";
        pub const FIND_ITEM_WITH: &CStr =
            c"?findItemWith@TabControl@MyGUI@@QEAAPEAVTabItem@2@AEBVUString@2@@Z";
        pub const GET_ITEM_COUNT: &CStr = c"?getItemCount@TabControl@MyGUI@@QEBA_KXZ";
        pub const SET_BUTTON_WIDTH_AT: &CStr = c"?setButtonWidthAt@TabControl@MyGUI@@QEAAX_KH@Z";
    }

    pub mod text_box {
        use super::CStr;

        pub const SET_CAPTION: &CStr = c"?setCaption@TextBox@MyGUI@@UEAAXAEBVUString@2@@Z";
        pub const SET_TEXT_COLOUR: &CStr = c"?setTextColour@TextBox@MyGUI@@UEAAXAEBUColour@2@@Z";
    }

    pub mod edit_text {
        use super::CStr;
        pub const APPLY_TEXT_COLOUR: &CStr =
            c"?_setTextColour@EditText@MyGUI@@AEAAXAEBUColour@2@@Z";
        pub const DESTROY: &CStr = c"??1EditText@MyGUI@@UEAA@XZ";
    }

    pub mod resource_skin {
        use super::CStr;
        pub const GET_RESOURCE_NAME: &CStr = c"?getResourceName@IResource@MyGUI@@QEBAAEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@XZ";
    }

    pub mod skin_item {
        use super::CStr;
        pub const CREATE_SKIN: &CStr =
            c"?_createSkinItem@SkinItem@MyGUI@@IEAAXPEAVResourceSkin@2@@Z";
        pub const SET_COLOUR: &CStr = c"?_setSkinItemColour@SkinItem@MyGUI@@IEAAXAEBUColour@2@@Z";
    }

    pub mod ustring {
        use super::CStr;

        pub const APPEND: &CStr = c"?append@UString@MyGUI@@QEAAAEAV12@PEBD_K@Z";
        pub const AS_UTF8: &CStr = c"?asUTF8@UString@MyGUI@@QEBAAEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@XZ";
        pub const AS_UTF8_C_STR: &CStr = c"?asUTF8_c_str@UString@MyGUI@@QEBAPEBDXZ";
        pub const CONSTRUCT_FROM_C_STR: &CStr = c"??0UString@MyGUI@@QEAA@PEBD@Z";
        pub const CONSTRUCT_FROM_STD_STRING: &CStr = c"??0UString@MyGUI@@QEAA@AEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@@Z";
        pub const COPY_CONSTRUCT: &CStr = c"??0UString@MyGUI@@QEAA@AEBV01@@Z";
        pub const DESTROY: &CStr = c"??1UString@MyGUI@@QEAA@XZ";
    }

    pub mod widget {
        use super::CStr;

        pub const CREATE_WIDGET_T: &CStr = c"?createWidgetT@Widget@MyGUI@@QEAAPEAV12@AEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@0AEBU?$TCoord@H@types@2@UAlign@2@0@Z";
        pub const GET_CHILD_AT: &CStr = c"?getChildAt@Widget@MyGUI@@QEAAPEAV12@_K@Z";
        pub const GET_CHILD_COUNT: &CStr = c"?getChildCount@Widget@MyGUI@@QEAA_KXZ";
        pub const GET_NAME: &CStr = c"?getName@Widget@MyGUI@@QEBAAEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@XZ";
        pub const GET_PARENT: &CStr = c"?getParent@Widget@MyGUI@@QEBAPEAV12@XZ";
        pub const GET_VISIBLE: &CStr = c"?getVisible@Widget@MyGUI@@QEBA_NXZ";
        pub const INITIALISE_SKIN: &CStr = c"?initialiseWidgetSkinBase@Widget@MyGUI@@AEAAPEBUWidgetInfo@2@PEAVResourceSkin@2@PEAVResourceLayout@2@@Z";
        pub const SHUTDOWN_SKIN: &CStr = c"?shutdownWidgetSkinBase@Widget@MyGUI@@AEAAXXZ";
        pub const SET_COORD: &CStr = c"?setCoord@Widget@MyGUI@@QEAAXHHHH@Z";
        pub const SET_COLOUR: &CStr = c"?setColour@Widget@MyGUI@@QEAAXAEBUColour@2@@Z";
        pub const SET_PROPERTY: &CStr = c"?setProperty@Widget@MyGUI@@QEAAXAEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@0@Z";
        pub const SET_ENABLED: &CStr = c"?setEnabled@Widget@MyGUI@@UEAAX_N@Z";
        pub const SET_VISIBLE: &CStr = c"?setVisible@Widget@MyGUI@@UEAAX_N@Z";
    }

    pub mod widget_input {
        use super::CStr;

        pub const RISE_MOUSE_BUTTON_CLICK: &CStr =
            c"?_riseMouseButtonClick@WidgetInput@MyGUI@@QEAAXXZ";
        pub const GET_ROOT_MOUSE_FOCUS: &CStr = c"?getRootMouseFocus@WidgetInput@MyGUI@@QEBA_NXZ";
    }
}

/// Exports from `OgreMain_x64.dll`.
pub mod ogre {
    use std::ffi::CStr;

    pub mod archive {
        use super::CStr;

        pub const GET_NAME: &CStr = c"?getName@Archive@Ogre@@QEBAAEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@XZ";
    }

    pub mod billboard_set {
        use super::CStr;

        pub const GET_BILLBOARDS_IN_WORLD_SPACE: &CStr =
            c"?getBillboardsInWorldSpace@BillboardSet@Ogre@@QEAA_NXZ";
        pub const GET_DEFAULT_HEIGHT: &CStr = c"?getDefaultHeight@BillboardSet@Ogre@@UEBAMXZ";
        pub const GET_DEFAULT_WIDTH: &CStr = c"?getDefaultWidth@BillboardSet@Ogre@@UEBAMXZ";
        pub const INJECT_BILLBOARD: &CStr =
            c"?injectBillboard@BillboardSet@Ogre@@QEAAXAEBVBillboard@2@PEBVCamera@2@@Z";
        pub const SET_DEFAULT_DIMENSIONS: &CStr =
            c"?setDefaultDimensions@BillboardSet@Ogre@@UEAAXMM@Z";
    }

    pub mod camera {
        use super::CStr;

        pub const GET_DERIVED_POSITION: &CStr =
            c"?getDerivedPosition@Camera@Ogre@@QEBAAEBVVector3@2@XZ";
    }

    pub mod codec {
        use super::CStr;

        pub const IS_CODEC_REGISTERED: &CStr = c"?isCodecRegistered@Codec@Ogre@@SA_NAEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@@Z";
        pub const REGISTER_CODEC: &CStr = c"?registerCodec@Codec@Ogre@@SAXPEAV12@@Z";
        pub const UNREGISTER_CODEC: &CStr = c"?unregisterCodec@Codec@Ogre@@SAXPEAV12@@Z";
    }

    pub mod dds_codec {
        use super::CStr;

        pub const DECODE: &CStr = c"?decode@DDSCodec@Ogre@@UEBA?AU?$pair@V?$SharedPtr@VMemoryDataStream@Ogre@@@Ogre@@V?$SharedPtr@VCodecData@Codec@Ogre@@@2@@std@@AEAV?$SharedPtr@VDataStream@Ogre@@@2@@Z";
    }

    pub mod file_system_archive {
        use super::CStr;

        pub const OPEN: &CStr = c"?open@FileSystemArchive@Ogre@@UEAA?AV?$SharedPtr@VDataStream@Ogre@@@2@AEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@_N@Z";
    }

    pub mod memory_data_stream {
        use super::CStr;

        pub const CONSTRUCT: &CStr = c"??0MemoryDataStream@Ogre@@QEAA@PEAX_K_N2@Z";
        pub const DESTROY: &CStr = c"??1MemoryDataStream@Ogre@@UEAA@XZ";
    }

    pub mod movable_object {
        use super::CStr;

        pub const GET_PARENT_SCENE_NODE: &CStr =
            c"?getParentSceneNode@MovableObject@Ogre@@QEBAPEAVSceneNode@2@XZ";
    }

    pub mod ned_pooling_impl {
        use super::CStr;

        pub const ALLOC_BYTES: &CStr = c"?allocBytes@NedPoolingImpl@Ogre@@SAPEAX_KPEBDH1@Z";
        pub const DEALLOC_BYTES: &CStr = c"?deallocBytes@NedPoolingImpl@Ogre@@SAXPEAX@Z";
    }

    pub mod node {
        use super::CStr;

        pub const GET_FULL_TRANSFORM: &CStr = c"?_getFullTransform@Node@Ogre@@QEBAAEBVMatrix4@2@XZ";
    }

    pub mod root {
        use super::CStr;

        pub const GET_SINGLETON_PTR: &CStr = c"?getSingletonPtr@Root@Ogre@@SAPEAV12@XZ";
    }

    pub mod shared_ptr {
        use super::CStr;

        pub const DESTROY_HARDWARE_COUNTER_BUFFER: &CStr =
            c"??1?$SharedPtr@VHardwareCounterBuffer@Ogre@@@Ogre@@QEAA@XZ";
    }
}

/// Exports from `Plugin_ParticleUniverse_x64.dll`.
pub mod particle_universe {
    use std::ffi::CStr;

    pub mod billboard_renderer {
        use super::CStr;

        pub const UPDATE_RENDER_QUEUE: &CStr = c"?_updateRenderQueue@BillboardRenderer@ParticleUniverse@@UEAAXPEAVRenderQueue@Ogre@@PEAVCamera@4@PEBV54@PEAVParticlePool@2@@Z";
    }

    pub mod particle_renderer {
        use super::CStr;

        pub const GET_PARENT_TECHNIQUE: &CStr = c"?getParentTechnique@ParticleRenderer@ParticleUniverse@@QEBAPEAVParticleTechnique@2@XZ";
    }

    pub mod particle_technique {
        use super::CStr;

        pub const GET_PARENT_SYSTEM: &CStr =
            c"?getParentSystem@ParticleTechnique@ParticleUniverse@@QEBAPEAVParticleSystem@2@XZ";
        pub const GET_MATERIAL_NAME: &CStr = c"?getMaterialName@ParticleTechnique@ParticleUniverse@@QEBAAEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@XZ";
    }
    pub mod particle_system {
        use super::CStr;
        pub const GET_TEMPLATE_NAME: &CStr = c"?getTemplateName@ParticleSystem@ParticleUniverse@@QEBAAEBV?$basic_string@DU?$char_traits@D@std@@V?$allocator@D@2@@std@@XZ";
    }
}
