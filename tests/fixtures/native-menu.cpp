// Headless hierarchy fixture using Kenshi's actual MyGUI DLL. Empty skins avoid
// a graphics device; this checks ownership, visibility and events, not rendering.
#include <algorithm>
#include <cstring>
#include <string>
#include <vector>
#define NOMINMAX
#include <windows.h>
#include "MyGUI_Gui.h"
#include "MyGUI_TabControl.h"
#include "MyGUI_TabItem.h"
#include "MyGUI_Button.h"
#include "MyGUI_RenderManager.h"
#include "MyGUI_DataManager.h"
#include "MyGUI_ResourceManager.h"
#include "MyGUI_LogManager.h"
#include "MyGUI_ITexture.h"
#include "MyGUI_TextBox.h"
#include "MyGUI_LayerManager.h"

class MemoryVertices : public MyGUI::IVertexBuffer {
public:
    std::vector<MyGUI::Vertex> vertices;
    void setVertexCount(size_t count) override { vertices.resize(count); }
    size_t getVertexCount() override { return vertices.size(); }
    MyGUI::Vertex* lock() override { return vertices.data(); }
    void unlock() override {}
};
class CaptureTarget : public MyGUI::IRenderTarget {
    MyGUI::RenderTargetInfo info;
public:
    std::vector<uint32_t> colours;
    void begin() override {}
    void end() override {}
    void doRender(MyGUI::IVertexBuffer* buffer, MyGUI::ITexture*, size_t count) override {
        const auto& data = static_cast<MemoryVertices*>(buffer)->vertices;
        for (size_t i = 0; i < count; ++i) colours.push_back(data.at(i).colour);
    }
    const MyGUI::RenderTargetInfo& getInfo() override { return info; }
};

class HeadlessTexture : public MyGUI::ITexture {
    std::string name = "fixture";
public:
    const std::string& getName() const override { return name; }
    void createManual(int, int, MyGUI::TextureUsage, MyGUI::PixelFormat) override {}
    void loadFromFile(const std::string&) override {}
    void saveToFile(const std::string&) override {}
    void destroy() override {}
    void* lock(MyGUI::TextureUsage) override { return nullptr; }
    void unlock() override {}
    bool isLocked() override { return false; }
    int getWidth() override { return 1920; }
    int getHeight() override { return 1080; }
    MyGUI::PixelFormat getFormat() override { return MyGUI::PixelFormat::R8G8B8A8; }
    MyGUI::TextureUsage getUsage() override { return MyGUI::TextureUsage::Static; }
    size_t getNumElemBytes() override { return 4; }
};

class HeadlessRender : public MyGUI::RenderManager {
    MyGUI::IntSize size{1280, 720};
    HeadlessTexture texture;
public:
    MyGUI::IVertexBuffer* createVertexBuffer() override { return new MemoryVertices; }
    void destroyVertexBuffer(MyGUI::IVertexBuffer* buffer) override { delete buffer; }
    MyGUI::ITexture* createTexture(const std::string&) override { return nullptr; }
    void destroyTexture(MyGUI::ITexture*) override {}
    MyGUI::ITexture* getTexture(const std::string&) override { return &texture; }
    const MyGUI::IntSize& getViewSize() const override { return size; }
    MyGUI::VertexColourType getVertexFormat() override { return MyGUI::VertexColourType::ColourARGB; }
};
class MemoryStream : public MyGUI::IDataStream {
    std::string text;
    size_t position = 0;
public:
    explicit MemoryStream(std::string value) : text(std::move(value)) {}
    bool eof() override { return position == text.size(); }
    size_t size() override { return text.size(); }
    void readline(std::string& out, MyGUI::Char delim) override {
        auto begin = position;
        while (!eof() && text[position] != delim) ++position;
        auto count = position - begin;
        if (!eof()) ++position;
        // This output belongs to VC++ 2010's CRT. Never let the fixture's
        // current CRT allocate/free its buffer. Both verified string ABIs have
        // a 16-byte union, then size and capacity; allocator policy differs.
        struct NativeString { union { char inline_data[16]; char* data; }; size_t size, capacity; };
        auto& native = reinterpret_cast<NativeString&>(out);
        auto crt = GetModuleHandleA("msvcr100.dll");
        auto alloc = reinterpret_cast<void* (*)(size_t)>(GetProcAddress(crt, "malloc"));
        auto free = reinterpret_cast<void (*)(void*)>(GetProcAddress(crt, "free"));
        if (count > native.capacity) {
            if (native.capacity >= 16) free(native.data);
            native.capacity = std::max(size_t(16), count);
            native.data = static_cast<char*>(alloc(native.capacity + 1));
        }
        auto output = native.capacity < 16 ? native.inline_data : native.data;
        memcpy(output, text.data() + begin, count); output[count] = 0; native.size = count;
    }
    size_t read(void* out, size_t count) override {
        count = std::min(count, text.size() - position);
        memcpy(out, text.data() + position, count); position += count; return count;
    }
};
class HeadlessData : public MyGUI::DataManager {
    std::vector<std::string> names;
    std::string path;
public:
    std::string resources;
    MyGUI::IDataStream* getData(const std::string&) override { return new MemoryStream(resources); }
    void freeData(MyGUI::IDataStream* data) override { delete data; }
    bool isDataExist(const std::string&) override { return true; }
    const std::vector<std::string>& getDataListNames(const std::string&) override { return names; }
    const std::string& getDataPath(const std::string&) override { return path; }
};
static MyGUI::Gui* gui;
static MyGUI::Widget* root;
static MyGUI::TabControl* tabs;
class DatapanelGUI;
class NativeText {
    alignas(8) unsigned char storage[64];
public:
    explicit NativeText(const char* value) {
        using Construct = void* (*)(void*, const char*);
        reinterpret_cast<Construct>(GetProcAddress(GetModuleHandleA("MyGUIEngine_x64.dll"), "??0UString@MyGUI@@QEAA@PEBD@Z"))(storage, value);
    }
    ~NativeText() {
        using Destroy = void (*)(void*);
        reinterpret_cast<Destroy>(GetProcAddress(GetModuleHandleA("MyGUIEngine_x64.dll"), "??1UString@MyGUI@@QEAA@XZ"))(storage);
    }
    const MyGUI::UString& get() const { return *reinterpret_cast<const MyGUI::UString*>(storage); }
};
static void make_options(const char* name) {
    root = gui->createWidgetT("Widget", "PanelEmpty", {0, 0, 800, 600}, MyGUI::Align::Default, "", name);
    tabs = static_cast<MyGUI::TabControl*>(root->createWidgetT("TabControl", "TestTabs", {0, 0, 760, 490}, MyGUI::Align::Default, std::string(name) + "_OptionsTab"));
    NativeText general("General"), mods("Mods");
    tabs->addItem(general.get(), reinterpret_cast<DatapanelGUI*>(0x12340));
    tabs->addItem(mods.get(), static_cast<DatapanelGUI*>(nullptr));
}
extern "C" __declspec(dllexport) void menu_setup(const char* resources) {
    auto log = new MyGUI::LogManager;
    log->createDefaultSource("fixture.log");
    log->setSTDOutputEnabled(false);
    new HeadlessRender;
    auto data = new HeadlessData; data->resources = resources;
    // Oversized storage: native construction owns all internal C++ containers.
    auto storage = new unsigned char[16384]();
    auto module = GetModuleHandleA("MyGUIEngine_x64.dll");
    using Construct = void* (*)(void*);
    reinterpret_cast<Construct>(GetProcAddress(module, "??0Gui@MyGUI@@QEAA@XZ"))(storage);
    gui = reinterpret_cast<MyGUI::Gui*>(storage);
    gui->initialise("");
    MyGUI::ResourceManager::getInstance().load("test.xml");
    make_options("First");
}
extern "C" __declspec(dllexport) void menu_frame() { gui->frameEvent(0.6f); }
static MyGUI::Any& native_data(size_t index) {
    // Verified shipped vector offset/record stride, not current compiler STL.
    auto records = *reinterpret_cast<unsigned char**>(reinterpret_cast<unsigned char*>(tabs) + 0x580);
    return *reinterpret_cast<MyGUI::Any*>(records + index * 0x58 + 0x50);
}
extern "C" __declspec(dllexport) bool menu_close_compatible() {
    // Kenshi casts and dereferences every tab's DatapanelGUI* data on close.
    // An empty Any fails that contract even though MyGUI accepts the tab.
    for (size_t i = 0; i < tabs->getItemCount(); ++i)
        if (!native_data(i).castType<DatapanelGUI*>(false)) return false;
    return true;
}
extern "C" __declspec(dllexport) uintptr_t menu_data(size_t i) {
    auto data = native_data(i).castType<DatapanelGUI*>(false);
    return data ? reinterpret_cast<uintptr_t>(*data) : uintptr_t(-1);
}
extern "C" __declspec(dllexport) void menu_hide(bool hidden) { root->setVisible(!hidden); }
extern "C" __declspec(dllexport) void menu_close() {
    tabs->removeAllItems(); root->setVisible(false);
}
extern "C" __declspec(dllexport) void menu_reopen() {
    NativeText general("General"), mods("Mods");
    tabs->addItem(general.get(), reinterpret_cast<DatapanelGUI*>(0x12340));
    tabs->addItem(mods.get(), static_cast<DatapanelGUI*>(nullptr));
    root->setVisible(true);
}
extern "C" __declspec(dllexport) void menu_rebuild() { gui->destroyWidget(root); make_options("Rebuilt"); }
extern "C" __declspec(dllexport) size_t menu_count() { return tabs->getItemCount(); }
extern "C" __declspec(dllexport) void* menu_tab() { NativeText name("REKENSHI-RUST"); return tabs->findItemWith(name.get()); }
extern "C" __declspec(dllexport) void menu_select(size_t index) { tabs->setIndexSelected(index); }
extern "C" __declspec(dllexport) void* menu_find(const char* name) { return gui->findWidgetT(name, false); }
extern "C" __declspec(dllexport) void* menu_parent(MyGUI::Widget* widget) { return widget->getParent(); }
extern "C" __declspec(dllexport) bool menu_visible(MyGUI::Widget* widget) {
    for (auto w = widget; w; w = w->getParent()) if (!w->getVisible()) return false;
    return true;
}
extern "C" __declspec(dllexport) void menu_click(MyGUI::Widget* widget) {
    // The shipped WidgetInput base is +0x80 (native click body subtracts it).
    // Current compiler STL sizes in old headers cannot calculate this offset.
    using Click = void (*)(void*);
    auto click = reinterpret_cast<Click>(GetProcAddress(GetModuleHandleA("MyGUIEngine_x64.dll"), "?_riseMouseButtonClick@WidgetInput@MyGUI@@QEAAXXZ"));
    click(reinterpret_cast<unsigned char*>(widget) + 0x80);
}
extern "C" __declspec(dllexport) bool menu_checked(MyGUI::Button* widget) { return widget->getStateSelected(); }
extern "C" __declspec(dllexport) int menu_width(MyGUI::Widget* widget) { return widget->getWidth(); }
extern "C" __declspec(dllexport) int menu_height(MyGUI::Widget* widget) { return widget->getHeight(); }
extern "C" __declspec(dllexport) int menu_top(MyGUI::Widget* widget) { return widget->getTop(); }
extern "C" __declspec(dllexport) bool menu_needs_mouse(MyGUI::Widget* widget) {
    using Getter = bool (*)(const void*);
    auto getter = reinterpret_cast<Getter>(GetProcAddress(GetModuleHandleA("MyGUIEngine_x64.dll"), "?getNeedMouseFocus@WidgetInput@MyGUI@@QEBA_NXZ"));
    return getter(reinterpret_cast<unsigned char*>(widget) + 0x80);
}
extern "C" __declspec(dllexport) bool menu_inherits_pick(MyGUI::Widget* widget) {
    using Getter = bool (*)(const void*);
    auto getter = reinterpret_cast<Getter>(GetProcAddress(GetModuleHandleA("MyGUIEngine_x64.dll"), "?getInheritsPick@WidgetInput@MyGUI@@QEBA_NXZ"));
    return getter(reinterpret_cast<unsigned char*>(widget) + 0x80);
}
extern "C" __declspec(dllexport) void menu_resize(int width, int height) { tabs->setSize(width, height); }
extern "C" __declspec(dllexport) void menu_shutdown() { gui->shutdown(); }

// Real subskins, without a render device. Use synthetic skin resources under
// Kenshi names to inspect native vertex tint/text state without loading textures.
extern "C" __declspec(dllexport) void theme_render_setup() {
    MyGUI::LayerManager::getInstance().createLayerAt("ThemeFixture", "OverlappedLayer", MyGUI::LayerManager::getInstance().getLayerCount());
}
extern "C" __declspec(dllexport) void* theme_create(const char* skin, const char* name) {
    return gui->createWidgetT("TextBox", skin, {0, 0, 100, 30}, MyGUI::Align::Default, "ThemeFixture", name);
}
extern "C" __declspec(dllexport) uint32_t theme_draw_colour() {
    CaptureTarget target;
    MyGUI::LayerManager::getInstance().renderToTarget(&target, true);
    return target.colours.empty() ? 0 : target.colours.front();
}
extern "C" __declspec(dllexport) uint32_t theme_draw_colour_at(size_t index) {
    CaptureTarget target;
    MyGUI::LayerManager::getInstance().renderToTarget(&target, true);
    return index < target.colours.size() ? target.colours[index] : 0;
}
extern "C" __declspec(dllexport) void theme_visible(MyGUI::Widget* widget, bool value) { widget->setVisible(value); }
extern "C" __declspec(dllexport) void theme_destroy(MyGUI::Widget* widget) { gui->destroyWidget(widget); }
extern "C" __declspec(dllexport) void theme_set_colour(MyGUI::Widget* widget, float r, float g, float b, float a) { widget->setColour({r, g, b, a}); }
extern "C" __declspec(dllexport) void theme_set_text(MyGUI::TextBox* widget, float r, float g, float b, float a) { widget->setTextColour({r, g, b, a}); }
extern "C" __declspec(dllexport) void theme_text_colour(MyGUI::TextBox* widget, float* out) {
    const auto& colour = widget->getTextColour();
    out[0] = colour.red; out[1] = colour.green; out[2] = colour.blue; out[3] = colour.alpha;
}
extern "C" __declspec(dllexport) void theme_set_enabled(MyGUI::Widget* widget, bool value) { widget->setEnabled(value); }
