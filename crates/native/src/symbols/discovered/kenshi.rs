//! Generated exports from `kenshi_x64.exe`.
//! An exported name does not establish a safe function signature.
use std::ffi::CStr;

pub const FILE: &str = "kenshi_x64.exe";
pub const SCANNED_SHA256: &str = "a596ab4e407c67b58599c54ffb32dc1bf2b64510cdebd3fa9359ef05a576aeb1";

/// code: `public: __cdecl CAkRegisteredObj::~CAkRegisteredObj(void) __ptr64`
pub const CAK_REGISTERED_OBJ_DESTROY_E5A7CACB1F49: &CStr = c"??1CAkRegisteredObj@@QEAA@XZ";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::AddBehavioralExtension(void (__cdecl*)(bool))`
pub const AK_SOUND_ENGINE_ADD_BEHAVIORAL_EXTENSION_9D90E1043CF6: &CStr = c"?AddBehavioralExtension@SoundEngine@AK@@YA?AW4AKRESULT@@P6AX_N@Z@Z";

/// code: `enum AKRESULT __cdecl AK::StreamMgr::AddLanguageChangeObserver(void (__cdecl*)(wchar_t const * __ptr64 const,void * __ptr64),void * __ptr64)`
pub const AK_STREAM_MGR_ADD_LANGUAGE_CHANGE_OBSERVER_AB1EA59D4F71: &CStr = c"?AddLanguageChangeObserver@StreamMgr@AK@@YA?AW4AKRESULT@@P6AXQEB_WPEAX@Z1@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::AddOutputCaptureMarker(char const * __ptr64)`
pub const AK_SOUND_ENGINE_ADD_OUTPUT_CAPTURE_MARKER_B222514CE730: &CStr = c"?AddOutputCaptureMarker@SoundEngine@AK@@YA?AW4AKRESULT@@PEBD@Z";

/// code: `enum AKRESULT __cdecl AK::MotionEngine::AddPlayerMotionDevice(unsigned char,unsigned long,unsigned long,void * __ptr64)`
pub const AK_MOTION_ENGINE_ADD_PLAYER_MOTION_DEVICE_8C38E92E1846: &CStr = c"?AddPlayerMotionDevice@MotionEngine@AK@@YA?AW4AKRESULT@@EKKPEAX@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::AddSecondaryOutput(unsigned long,enum AkAudioOutputType,unsigned long)`
pub const AK_SOUND_ENGINE_ADD_SECONDARY_OUTPUT_0681313D0C6B: &CStr = c"?AddSecondaryOutput@SoundEngine@AK@@YA?AW4AKRESULT@@KW4AkAudioOutputType@@K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::DynamicSequence::Break(unsigned long)`
pub const AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_BREAK_22FC59F23D9B: &CStr = c"?Break@DynamicSequence@SoundEngine@AK@@YA?AW4AKRESULT@@K@Z";

/// code: `void __cdecl AK::SoundEngine::CancelBankCallbackCookie(void * __ptr64)`
pub const AK_SOUND_ENGINE_CANCEL_BANK_CALLBACK_COOKIE_54ADF0EA1A24: &CStr = c"?CancelBankCallbackCookie@SoundEngine@AK@@YAXPEAX@Z";

/// code: `void __cdecl AK::SoundEngine::CancelEventCallback(unsigned long)`
pub const AK_SOUND_ENGINE_CANCEL_EVENT_CALLBACK_B4C546A9D8B6: &CStr = c"?CancelEventCallback@SoundEngine@AK@@YAXK@Z";

/// code: `void __cdecl AK::SoundEngine::CancelEventCallbackCookie(void * __ptr64)`
pub const AK_SOUND_ENGINE_CANCEL_EVENT_CALLBACK_COOKIE_7164FF264B70: &CStr = c"?CancelEventCallbackCookie@SoundEngine@AK@@YAXPEAX@Z";

/// code: `enum AKRESULT __cdecl AK::MemoryMgr::CheckPoolId(long)`
pub const AK_MEMORY_MGR_CHECK_POOL_ID_18C7816585B0: &CStr = c"?CheckPoolId@MemoryMgr@AK@@YA?AW4AKRESULT@@J@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::ClearBanks(void)`
pub const AK_SOUND_ENGINE_CLEAR_BANKS_41986F9F4ECE: &CStr = c"?ClearBanks@SoundEngine@AK@@YA?AW4AKRESULT@@XZ";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::ClearPreparedEvents(void)`
pub const AK_SOUND_ENGINE_CLEAR_PREPARED_EVENTS_15ED182A03AE: &CStr = c"?ClearPreparedEvents@SoundEngine@AK@@YA?AW4AKRESULT@@XZ";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::DynamicSequence::Close(unsigned long)`
pub const AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_CLOSE_AB846A881AC3: &CStr = c"?Close@DynamicSequence@SoundEngine@AK@@YA?AW4AKRESULT@@K@Z";

/// code: `class AK::IAkStreamMgr * __ptr64 __cdecl AK::StreamMgr::Create(struct AkStreamMgrSettings const & __ptr64)`
pub const AK_STREAM_MGR_CREATE_F996534A33FC: &CStr = c"?Create@StreamMgr@AK@@YAPEAVIAkStreamMgr@2@AEBUAkStreamMgrSettings@@@Z";

/// code: `unsigned long __cdecl AK::StreamMgr::CreateDevice(struct AkDeviceSettings const & __ptr64,class AK::StreamMgr::IAkLowLevelIOHook * __ptr64)`
pub const AK_STREAM_MGR_CREATE_DEVICE_FB2486DE0368: &CStr = c"?CreateDevice@StreamMgr@AK@@YAKAEBUAkDeviceSettings@@PEAVIAkLowLevelIOHook@12@@Z";

/// code: `long __cdecl AK::MemoryMgr::CreatePool(void * __ptr64,unsigned long,unsigned long,unsigned long,unsigned long)`
pub const AK_MEMORY_MGR_CREATE_POOL_18CA554CEB75: &CStr = c"?CreatePool@MemoryMgr@AK@@YAJPEAXKKKK@Z";

/// code: `enum AKRESULT __cdecl AK::StreamMgr::DestroyDevice(unsigned long)`
pub const AK_STREAM_MGR_DESTROY_DEVICE_93CA085272CF: &CStr = c"?DestroyDevice@StreamMgr@AK@@YA?AW4AKRESULT@@K@Z";

/// code: `enum AKRESULT __cdecl AK::MemoryMgr::DestroyPool(long)`
pub const AK_MEMORY_MGR_DESTROY_POOL_D2CEDFDFAE6A: &CStr = c"?DestroyPool@MemoryMgr@AK@@YA?AW4AKRESULT@@J@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::ExecuteActionOnEvent(unsigned long,enum AK::SoundEngine::AkActionOnEventType,unsigned __int64,long,enum AkCurveInterpolation,unsigned long)`
pub const AK_SOUND_ENGINE_EXECUTE_ACTION_ON_EVENT_CFF0C23D5775: &CStr = c"?ExecuteActionOnEvent@SoundEngine@AK@@YA?AW4AKRESULT@@KW4AkActionOnEventType@12@_KJW4AkCurveInterpolation@@K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::ExecuteActionOnEvent(char const * __ptr64,enum AK::SoundEngine::AkActionOnEventType,unsigned __int64,long,enum AkCurveInterpolation,unsigned long)`
pub const AK_SOUND_ENGINE_EXECUTE_ACTION_ON_EVENT_BC4592C854DC: &CStr = c"?ExecuteActionOnEvent@SoundEngine@AK@@YA?AW4AKRESULT@@PEBDW4AkActionOnEventType@12@_KJW4AkCurveInterpolation@@K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::ExecuteActionOnEvent(wchar_t const * __ptr64,enum AK::SoundEngine::AkActionOnEventType,unsigned __int64,long,enum AkCurveInterpolation,unsigned long)`
pub const AK_SOUND_ENGINE_EXECUTE_ACTION_ON_EVENT_6E34E4AC5DFA: &CStr = c"?ExecuteActionOnEvent@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_WW4AkActionOnEventType@12@_KJW4AkCurveInterpolation@@K@Z";

/// code: `enum AKRESULT __cdecl AK::MemoryMgr::Falign(long,void * __ptr64)`
pub const AK_MEMORY_MGR_FALIGN_A3C720AD9801: &CStr = c"?Falign@MemoryMgr@AK@@YA?AW4AKRESULT@@JPEAX@Z";

/// code: `void __cdecl AK::StreamMgr::FlushAllCaches(void)`
pub const AK_STREAM_MGR_FLUSH_ALL_CACHES_4BF1A9544EB2: &CStr = c"?FlushAllCaches@StreamMgr@AK@@YAXXZ";

/// code: `enum AKRESULT __cdecl AK::MemoryMgr::Free(long,void * __ptr64)`
pub const AK_MEMORY_MGR_FREE_A86D584F2BF9: &CStr = c"?Free@MemoryMgr@AK@@YA?AW4AKRESULT@@JPEAX@Z";

/// code: `void * __ptr64 __cdecl AK::MemoryMgr::GetBlock(long)`
pub const AK_MEMORY_MGR_GET_BLOCK_A253FF7FAE5F: &CStr = c"?GetBlock@MemoryMgr@AK@@YAPEAXJ@Z";

/// code: `unsigned long __cdecl AK::MemoryMgr::GetBlockSize(long)`
pub const AK_MEMORY_MGR_GET_BLOCK_SIZE_C6483D487C16: &CStr = c"?GetBlockSize@MemoryMgr@AK@@YAKJ@Z";

/// code: `wchar_t const * __ptr64 __cdecl AK::StreamMgr::GetCurrentLanguage(void)`
pub const AK_STREAM_MGR_GET_CURRENT_LANGUAGE_FBBF37CFBCF1: &CStr = c"?GetCurrentLanguage@StreamMgr@AK@@YAPEB_WXZ";

/// code: `void __cdecl AK::StreamMgr::GetDefaultDeviceSettings(struct AkDeviceSettings & __ptr64)`
pub const AK_STREAM_MGR_GET_DEFAULT_DEVICE_SETTINGS_C165EA988679: &CStr = c"?GetDefaultDeviceSettings@StreamMgr@AK@@YAXAEAUAkDeviceSettings@@@Z";

/// code: `void __cdecl AK::MusicEngine::GetDefaultInitSettings(struct AkMusicSettings & __ptr64)`
pub const AK_MUSIC_ENGINE_GET_DEFAULT_INIT_SETTINGS_E232787366BF: &CStr = c"?GetDefaultInitSettings@MusicEngine@AK@@YAXAEAUAkMusicSettings@@@Z";

/// code: `void __cdecl AK::SoundEngine::GetDefaultInitSettings(struct AkInitSettings & __ptr64)`
pub const AK_SOUND_ENGINE_GET_DEFAULT_INIT_SETTINGS_1276F3B87CB6: &CStr = c"?GetDefaultInitSettings@SoundEngine@AK@@YAXAEAUAkInitSettings@@@Z";

/// code: `void __cdecl AK::SoundEngine::GetDefaultPlatformInitSettings(struct AkPlatformInitSettings & __ptr64)`
pub const AK_SOUND_ENGINE_GET_DEFAULT_PLATFORM_INIT_SETTINGS_BC90DCDE222D: &CStr = c"?GetDefaultPlatformInitSettings@SoundEngine@AK@@YAXAEAUAkPlatformInitSettings@@@Z";

/// code: `void __cdecl AK::StreamMgr::GetDefaultSettings(struct AkStreamMgrSettings & __ptr64)`
pub const AK_STREAM_MGR_GET_DEFAULT_SETTINGS_A4B33701A6B3: &CStr = c"?GetDefaultSettings@StreamMgr@AK@@YAXAEAUAkStreamMgrSettings@@@Z";

/// code: `struct IDirectSound8 * __ptr64 __cdecl AK::GetDirectSoundInstance(void)`
pub const AK_GET_DIRECT_SOUND_INSTANCE_A0C28141EC61: &CStr = c"?GetDirectSoundInstance@AK@@YAPEAUIDirectSound8@@XZ";

/// code: `class AK::StreamMgr::IAkFileLocationResolver * __ptr64 __cdecl AK::StreamMgr::GetFileLocationResolver(void)`
pub const AK_STREAM_MGR_GET_FILE_LOCATION_RESOLVER_0DD219FCC4E9: &CStr = c"?GetFileLocationResolver@StreamMgr@AK@@YAPEAVIAkFileLocationResolver@12@XZ";

/// code: `unsigned long __cdecl AK::SoundEngine::GetIDFromString(char const * __ptr64)`
pub const AK_SOUND_ENGINE_GET_IDFROM_STRING_508DCB26719F: &CStr = c"?GetIDFromString@SoundEngine@AK@@YAKPEBD@Z";

/// code: `unsigned long __cdecl AK::SoundEngine::GetIDFromString(wchar_t const * __ptr64)`
pub const AK_SOUND_ENGINE_GET_IDFROM_STRING_69A82739C07B: &CStr = c"?GetIDFromString@SoundEngine@AK@@YAKPEB_W@Z";

/// code: `long __cdecl AK::MemoryMgr::GetMaxPools(void)`
pub const AK_MEMORY_MGR_GET_MAX_POOLS_865D9EEDACE0: &CStr = c"?GetMaxPools@MemoryMgr@AK@@YAJXZ";

/// code: `long __cdecl AK::MemoryMgr::GetNumPools(void)`
pub const AK_MEMORY_MGR_GET_NUM_POOLS_B63C9E35D262: &CStr = c"?GetNumPools@MemoryMgr@AK@@YAJXZ";

/// code: `public: class CAkRegisteredObj * __ptr64 __cdecl CAkRegistryMgr::GetObjAndAddref(unsigned __int64) __ptr64`
pub const CAK_REGISTRY_MGR_GET_OBJ_AND_ADDREF_4C20D18EE446: &CStr = c"?GetObjAndAddref@CAkRegistryMgr@@QEAAPEAVCAkRegisteredObj@@_K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::GetPanningRule(enum AkPanningRule & __ptr64,enum AkAudioOutputType,unsigned long)`
pub const AK_SOUND_ENGINE_GET_PANNING_RULE_7578D99D51E1: &CStr = c"?GetPanningRule@SoundEngine@AK@@YA?AW4AKRESULT@@AEAW4AkPanningRule@@W4AkAudioOutputType@@K@Z";

/// code: `enum AKRESULT __cdecl AK::MusicEngine::GetPlayingSegmentInfo(unsigned long,struct AkSegmentInfo & __ptr64,bool)`
pub const AK_MUSIC_ENGINE_GET_PLAYING_SEGMENT_INFO_9F952566AF7C: &CStr = c"?GetPlayingSegmentInfo@MusicEngine@AK@@YA?AW4AKRESULT@@KAEAUAkSegmentInfo@@_N@Z";

/// code: `enum AkMemPoolAttributes __cdecl AK::MemoryMgr::GetPoolAttributes(long)`
pub const AK_MEMORY_MGR_GET_POOL_ATTRIBUTES_D599CDDA0B18: &CStr = c"?GetPoolAttributes@MemoryMgr@AK@@YA?AW4AkMemPoolAttributes@@J@Z";

/// code: `long __cdecl AK::StreamMgr::GetPoolID(void)`
pub const AK_STREAM_MGR_GET_POOL_ID_0E1D100CC585: &CStr = c"?GetPoolID@StreamMgr@AK@@YAJXZ";

/// code: `void __cdecl AK::MemoryMgr::GetPoolMemoryUsed(long,struct AK::MemoryMgr::PoolMemInfo & __ptr64)`
pub const AK_MEMORY_MGR_GET_POOL_MEMORY_USED_17407D3262F3: &CStr = c"?GetPoolMemoryUsed@MemoryMgr@AK@@YAXJAEAUPoolMemInfo@12@@Z";

/// code: `wchar_t * __ptr64 __cdecl AK::MemoryMgr::GetPoolName(long)`
pub const AK_MEMORY_MGR_GET_POOL_NAME_A42C13004EF9: &CStr = c"?GetPoolName@MemoryMgr@AK@@YAPEA_WJ@Z";

/// code: `enum AKRESULT __cdecl AK::MemoryMgr::GetPoolStats(long,struct AK::MemoryMgr::PoolStats & __ptr64)`
pub const AK_MEMORY_MGR_GET_POOL_STATS_87DDF30725DC: &CStr = c"?GetPoolStats@MemoryMgr@AK@@YA?AW4AKRESULT@@JAEAUPoolStats@12@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::GetSourcePlayPosition(unsigned long,long * __ptr64,bool)`
pub const AK_SOUND_ENGINE_GET_SOURCE_PLAY_POSITION_4D5BABCF29B9: &CStr = c"?GetSourcePlayPosition@SoundEngine@AK@@YA?AW4AKRESULT@@KPEAJ_N@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::GetSourceStreamBuffering(unsigned long,long & __ptr64,bool & __ptr64)`
pub const AK_SOUND_ENGINE_GET_SOURCE_STREAM_BUFFERING_7C3266A34862: &CStr = c"?GetSourceStreamBuffering@SoundEngine@AK@@YA?AW4AKRESULT@@KAEAJAEA_N@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::GetSpeakerAngles(float * __ptr64,unsigned long & __ptr64,float & __ptr64,enum AkAudioOutputType,unsigned long)`
pub const AK_SOUND_ENGINE_GET_SPEAKER_ANGLES_A0BFCAB16944: &CStr = c"?GetSpeakerAngles@SoundEngine@AK@@YA?AW4AKRESULT@@PEAMAEAKAEAMW4AkAudioOutputType@@K@Z";

/// code: `struct AkChannelConfig __cdecl AK::SoundEngine::GetSpeakerConfiguration(enum AkAudioOutputType,unsigned long)`
pub const AK_SOUND_ENGINE_GET_SPEAKER_CONFIGURATION_92856BED21DA: &CStr = c"?GetSpeakerConfiguration@SoundEngine@AK@@YA?AUAkChannelConfig@@W4AkAudioOutputType@@K@Z";

/// code: `long __cdecl AK::Monitor::GetTimeStamp(void)`
pub const AK_MONITOR_GET_TIME_STAMP_76911C27D7C7: &CStr = c"?GetTimeStamp@Monitor@AK@@YAJXZ";

/// code: `struct IXAudio2 * __ptr64 __cdecl AK::GetWwiseXAudio2Interface(void)`
pub const AK_GET_WWISE_XAUDIO2_INTERFACE_3BC5E1D5B37B: &CStr = c"?GetWwiseXAudio2Interface@AK@@YAPEAUIXAudio2@@XZ";

/// code: `enum AKRESULT __cdecl AK::MusicEngine::Init(struct AkMusicSettings * __ptr64)`
pub const AK_MUSIC_ENGINE_INIT_FCD348A45990: &CStr = c"?Init@MusicEngine@AK@@YA?AW4AKRESULT@@PEAUAkMusicSettings@@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::Init(struct AkInitSettings * __ptr64,struct AkPlatformInitSettings * __ptr64)`
pub const AK_SOUND_ENGINE_INIT_0D2F1CAAA52B: &CStr = c"?Init@SoundEngine@AK@@YA?AW4AKRESULT@@PEAUAkInitSettings@@PEAUAkPlatformInitSettings@@@Z";

/// code: `bool __cdecl AK::MemoryMgr::IsInitialized(void)`
pub const AK_MEMORY_MGR_IS_INITIALIZED_1BEECB520943: &CStr = c"?IsInitialized@MemoryMgr@AK@@YA_NXZ";

/// code: `bool __cdecl AK::SoundEngine::IsInitialized(void)`
pub const AK_SOUND_ENGINE_IS_INITIALIZED_1E2CA3D17A16: &CStr = c"?IsInitialized@SoundEngine@AK@@YA_NXZ";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::LoadBank(unsigned long,long)`
pub const AK_SOUND_ENGINE_LOAD_BANK_29A29AAB4F6D: &CStr = c"?LoadBank@SoundEngine@AK@@YA?AW4AKRESULT@@KJ@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::LoadBank(unsigned long,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64,long)`
pub const AK_SOUND_ENGINE_LOAD_BANK_2FAE5ABAB905: &CStr = c"?LoadBank@SoundEngine@AK@@YA?AW4AKRESULT@@KP6AXKPEBXW43@JPEAX@Z2J@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::LoadBank(char const * __ptr64,long,unsigned long & __ptr64)`
pub const AK_SOUND_ENGINE_LOAD_BANK_AD65959920C3: &CStr = c"?LoadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEBDJAEAK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::LoadBank(char const * __ptr64,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64,long,unsigned long & __ptr64)`
pub const AK_SOUND_ENGINE_LOAD_BANK_6FFB2BE7212A: &CStr = c"?LoadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEBDP6AXKPEBXW43@JPEAX@Z3JAEAK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::LoadBank(void const * __ptr64,unsigned long,unsigned long & __ptr64)`
pub const AK_SOUND_ENGINE_LOAD_BANK_1324DD9473F0: &CStr = c"?LoadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEBXKAEAK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::LoadBank(void const * __ptr64,unsigned long,long,unsigned long & __ptr64)`
pub const AK_SOUND_ENGINE_LOAD_BANK_9B1E1B7BCF00: &CStr = c"?LoadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEBXKJAEAK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::LoadBank(void const * __ptr64,unsigned long,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64,unsigned long & __ptr64)`
pub const AK_SOUND_ENGINE_LOAD_BANK_88EBC790AC04: &CStr = c"?LoadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEBXKP6AXK0W43@JPEAX@Z2AEAK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::LoadBank(void const * __ptr64,unsigned long,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64,long,unsigned long & __ptr64)`
pub const AK_SOUND_ENGINE_LOAD_BANK_509CF55D157D: &CStr = c"?LoadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEBXKP6AXK0W43@JPEAX@Z2JAEAK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::LoadBank(wchar_t const * __ptr64,long,unsigned long & __ptr64)`
pub const AK_SOUND_ENGINE_LOAD_BANK_7B66A39B71FE: &CStr = c"?LoadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_WJAEAK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::LoadBank(wchar_t const * __ptr64,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64,long,unsigned long & __ptr64)`
pub const AK_SOUND_ENGINE_LOAD_BANK_7151896015F4: &CStr = c"?LoadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_WP6AXKPEBXW43@JPEAX@Z3JAEAK@Z";

/// code: `class AK::SoundEngine::DynamicSequence::Playlist * __ptr64 __cdecl AK::SoundEngine::DynamicSequence::LockPlaylist(unsigned long)`
pub const AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_LOCK_PLAYLIST_5337987BA7E1: &CStr = c"?LockPlaylist@DynamicSequence@SoundEngine@AK@@YAPEAVPlaylist@123@K@Z";

/// code: `void * __ptr64 __cdecl AK::MemoryMgr::Malign(long,unsigned __int64,unsigned long)`
pub const AK_MEMORY_MGR_MALIGN_AC3524AEA0A2: &CStr = c"?Malign@MemoryMgr@AK@@YAPEAXJ_KK@Z";

/// code: `void * __ptr64 __cdecl AK::MemoryMgr::Malloc(long,unsigned __int64)`
pub const AK_MEMORY_MGR_MALLOC_DB1E34830BA8: &CStr = c"?Malloc@MemoryMgr@AK@@YAPEAXJ_K@Z";

/// code: `void __cdecl AK::SoundEngine::MuteBackgroundMusic(bool)`
pub const AK_SOUND_ENGINE_MUTE_BACKGROUND_MUSIC_917D3D285058: &CStr = c"?MuteBackgroundMusic@SoundEngine@AK@@YAX_N@Z";

/// code: `unsigned long __cdecl AK::SoundEngine::DynamicSequence::Open(unsigned __int64,unsigned long,void (__cdecl*)(enum AkCallbackType,struct AkCallbackInfo * __ptr64),void * __ptr64,enum AK::SoundEngine::DynamicSequence::DynamicSequenceType)`
pub const AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_OPEN_47F58EE88370: &CStr = c"?Open@DynamicSequence@SoundEngine@AK@@YAK_KKP6AXW4AkCallbackType@@PEAUAkCallbackInfo@@@ZPEAXW4DynamicSequenceType@123@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::DynamicSequence::Pause(unsigned long,long,enum AkCurveInterpolation)`
pub const AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_PAUSE_46B932BAD62B: &CStr = c"?Pause@DynamicSequence@SoundEngine@AK@@YA?AW4AKRESULT@@KJW4AkCurveInterpolation@@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::DynamicSequence::Play(unsigned long,long,enum AkCurveInterpolation)`
pub const AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_PLAY_191E761E4A09: &CStr = c"?Play@DynamicSequence@SoundEngine@AK@@YA?AW4AKRESULT@@KJW4AkCurveInterpolation@@@Z";

/// code: `unsigned long __cdecl AK::SoundEngine::PlaySourcePlugin(unsigned long,unsigned long,unsigned __int64)`
pub const AK_SOUND_ENGINE_PLAY_SOURCE_PLUGIN_D8322C98F830: &CStr = c"?PlaySourcePlugin@SoundEngine@AK@@YAKKK_K@Z";

/// code: `enum AKRESULT __cdecl AK::Monitor::PostCode(enum AK::Monitor::ErrorCode,enum AK::Monitor::ErrorLevel)`
pub const AK_MONITOR_POST_CODE_5E81F0658BC7: &CStr = c"?PostCode@Monitor@AK@@YA?AW4AKRESULT@@W4ErrorCode@12@W4ErrorLevel@12@@Z";

/// code: `unsigned long __cdecl AK::SoundEngine::PostEvent(unsigned long,unsigned __int64,unsigned long,void (__cdecl*)(enum AkCallbackType,struct AkCallbackInfo * __ptr64),void * __ptr64,unsigned long,struct AkExternalSourceInfo * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_POST_EVENT_9C7636B50778: &CStr = c"?PostEvent@SoundEngine@AK@@YAKK_KKP6AXW4AkCallbackType@@PEAUAkCallbackInfo@@@ZPEAXKPEAUAkExternalSourceInfo@@K@Z";

/// code: `unsigned long __cdecl AK::SoundEngine::PostEvent(char const * __ptr64,unsigned __int64,unsigned long,void (__cdecl*)(enum AkCallbackType,struct AkCallbackInfo * __ptr64),void * __ptr64,unsigned long,struct AkExternalSourceInfo * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_POST_EVENT_0EA4B18D2597: &CStr = c"?PostEvent@SoundEngine@AK@@YAKPEBD_KKP6AXW4AkCallbackType@@PEAUAkCallbackInfo@@@ZPEAXKPEAUAkExternalSourceInfo@@K@Z";

/// code: `unsigned long __cdecl AK::SoundEngine::PostEvent(wchar_t const * __ptr64,unsigned __int64,unsigned long,void (__cdecl*)(enum AkCallbackType,struct AkCallbackInfo * __ptr64),void * __ptr64,unsigned long,struct AkExternalSourceInfo * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_POST_EVENT_D8C0ED3B4722: &CStr = c"?PostEvent@SoundEngine@AK@@YAKPEB_W_KKP6AXW4AkCallbackType@@PEAUAkCallbackInfo@@@ZPEAXKPEAUAkExternalSourceInfo@@K@Z";

/// code: `enum AKRESULT __cdecl AK::Monitor::PostString(char const * __ptr64,enum AK::Monitor::ErrorLevel)`
pub const AK_MONITOR_POST_STRING_19871C7F5148: &CStr = c"?PostString@Monitor@AK@@YA?AW4AKRESULT@@PEBDW4ErrorLevel@12@@Z";

/// code: `enum AKRESULT __cdecl AK::Monitor::PostString(wchar_t const * __ptr64,enum AK::Monitor::ErrorLevel)`
pub const AK_MONITOR_POST_STRING_A5A143AB7935: &CStr = c"?PostString@Monitor@AK@@YA?AW4AKRESULT@@PEB_WW4ErrorLevel@12@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PostTrigger(unsigned long,unsigned __int64)`
pub const AK_SOUND_ENGINE_POST_TRIGGER_54E7EA3ED4DB: &CStr = c"?PostTrigger@SoundEngine@AK@@YA?AW4AKRESULT@@K_K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PostTrigger(char const * __ptr64,unsigned __int64)`
pub const AK_SOUND_ENGINE_POST_TRIGGER_1B80238DC1E5: &CStr = c"?PostTrigger@SoundEngine@AK@@YA?AW4AKRESULT@@PEBD_K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PostTrigger(wchar_t const * __ptr64,unsigned __int64)`
pub const AK_SOUND_ENGINE_POST_TRIGGER_1BD23F19E5B0: &CStr = c"?PostTrigger@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_W_K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareBank(enum AK::SoundEngine::PreparationType,unsigned long,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64,enum AK::SoundEngine::AkBankContent)`
pub const AK_SOUND_ENGINE_PREPARE_BANK_7009801B3A20: &CStr = c"?PrepareBank@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@KP6AXKPEBXW43@JPEAX@Z3W4AkBankContent@12@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareBank(enum AK::SoundEngine::PreparationType,unsigned long,enum AK::SoundEngine::AkBankContent)`
pub const AK_SOUND_ENGINE_PREPARE_BANK_75145F2DB484: &CStr = c"?PrepareBank@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@KW4AkBankContent@12@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareBank(enum AK::SoundEngine::PreparationType,char const * __ptr64,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64,enum AK::SoundEngine::AkBankContent)`
pub const AK_SOUND_ENGINE_PREPARE_BANK_B418D4A8520A: &CStr = c"?PrepareBank@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@PEBDP6AXKPEBXW43@JPEAX@Z4W4AkBankContent@12@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareBank(enum AK::SoundEngine::PreparationType,char const * __ptr64,enum AK::SoundEngine::AkBankContent)`
pub const AK_SOUND_ENGINE_PREPARE_BANK_DD1553532714: &CStr = c"?PrepareBank@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@PEBDW4AkBankContent@12@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareBank(enum AK::SoundEngine::PreparationType,wchar_t const * __ptr64,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64,enum AK::SoundEngine::AkBankContent)`
pub const AK_SOUND_ENGINE_PREPARE_BANK_8C669803C604: &CStr = c"?PrepareBank@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@PEB_WP6AXKPEBXW43@JPEAX@Z4W4AkBankContent@12@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareBank(enum AK::SoundEngine::PreparationType,wchar_t const * __ptr64,enum AK::SoundEngine::AkBankContent)`
pub const AK_SOUND_ENGINE_PREPARE_BANK_FA0B3C43DD2D: &CStr = c"?PrepareBank@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@PEB_WW4AkBankContent@12@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareEvent(enum AK::SoundEngine::PreparationType,unsigned long * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_PREPARE_EVENT_7618CB54A057: &CStr = c"?PrepareEvent@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@PEAKK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareEvent(enum AK::SoundEngine::PreparationType,unsigned long * __ptr64,unsigned long,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64)`
pub const AK_SOUND_ENGINE_PREPARE_EVENT_9EB838DF9A50: &CStr = c"?PrepareEvent@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@PEAKKP6AXKPEBXW43@JPEAX@Z4@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareEvent(enum AK::SoundEngine::PreparationType,char const * __ptr64 * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_PREPARE_EVENT_896E426295F8: &CStr = c"?PrepareEvent@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@PEAPEBDK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareEvent(enum AK::SoundEngine::PreparationType,char const * __ptr64 * __ptr64,unsigned long,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64)`
pub const AK_SOUND_ENGINE_PREPARE_EVENT_56A420F01080: &CStr = c"?PrepareEvent@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@PEAPEBDKP6AXKPEBXW43@JPEAX@Z4@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareEvent(enum AK::SoundEngine::PreparationType,wchar_t const * __ptr64 * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_PREPARE_EVENT_56C05AADF115: &CStr = c"?PrepareEvent@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@PEAPEB_WK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareEvent(enum AK::SoundEngine::PreparationType,wchar_t const * __ptr64 * __ptr64,unsigned long,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64)`
pub const AK_SOUND_ENGINE_PREPARE_EVENT_662614AE28AA: &CStr = c"?PrepareEvent@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@PEAPEB_WKP6AXKPEBXW43@JPEAX@Z4@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareGameSyncs(enum AK::SoundEngine::PreparationType,enum AkGroupType,unsigned long,unsigned long * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_C0F882472841: &CStr = c"?PrepareGameSyncs@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@W4AkGroupType@@KPEAKK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareGameSyncs(enum AK::SoundEngine::PreparationType,enum AkGroupType,unsigned long,unsigned long * __ptr64,unsigned long,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64)`
pub const AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_1B876E474B24: &CStr = c"?PrepareGameSyncs@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@W4AkGroupType@@KPEAKKP6AXKPEBXW43@JPEAX@Z5@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareGameSyncs(enum AK::SoundEngine::PreparationType,enum AkGroupType,char const * __ptr64,char const * __ptr64 * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_ED73BA38926C: &CStr = c"?PrepareGameSyncs@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@W4AkGroupType@@PEBDPEAPEBDK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareGameSyncs(enum AK::SoundEngine::PreparationType,enum AkGroupType,char const * __ptr64,char const * __ptr64 * __ptr64,unsigned long,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64)`
pub const AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_528043A8B362: &CStr = c"?PrepareGameSyncs@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@W4AkGroupType@@PEBDPEAPEBDKP6AXKPEBXW43@JPEAX@Z6@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareGameSyncs(enum AK::SoundEngine::PreparationType,enum AkGroupType,wchar_t const * __ptr64,wchar_t const * __ptr64 * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_1B5F3FAEBBF6: &CStr = c"?PrepareGameSyncs@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@W4AkGroupType@@PEB_WPEAPEB_WK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::PrepareGameSyncs(enum AK::SoundEngine::PreparationType,enum AkGroupType,wchar_t const * __ptr64,wchar_t const * __ptr64 * __ptr64,unsigned long,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64)`
pub const AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_31A0FC352D2C: &CStr = c"?PrepareGameSyncs@SoundEngine@AK@@YA?AW4AKRESULT@@W4PreparationType@12@W4AkGroupType@@PEB_WPEAPEB_WKP6AXKPEBXW43@JPEAX@Z6@Z";

/// code: `unsigned long __cdecl AkMonitorData::RealSizeof(struct AkMonitorData::MonitorDataItem const & __ptr64)`
pub const AK_MONITOR_DATA_REAL_SIZEOF_8E3F0E572D6F: &CStr = c"?RealSizeof@AkMonitorData@@YAKAEBUMonitorDataItem@1@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::RegisterBusMeteringCallback(unsigned long,void (__cdecl*)(class AK::IAkMetering * __ptr64,struct AkChannelConfig,enum AkMeteringFlags),enum AkMeteringFlags)`
pub const AK_SOUND_ENGINE_REGISTER_BUS_METERING_CALLBACK_6D77D3742B8D: &CStr = c"?RegisterBusMeteringCallback@SoundEngine@AK@@YA?AW4AKRESULT@@KP6AXPEAVIAkMetering@2@UAkChannelConfig@@W4AkMeteringFlags@@@Z2@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::RegisterBusVolumeCallback(unsigned long,void (__cdecl*)(struct AkSpeakerVolumeMatrixCallbackInfo * __ptr64))`
pub const AK_SOUND_ENGINE_REGISTER_BUS_VOLUME_CALLBACK_C43FF1BB9E63: &CStr = c"?RegisterBusVolumeCallback@SoundEngine@AK@@YA?AW4AKRESULT@@KP6AXPEAUAkSpeakerVolumeMatrixCallbackInfo@@@Z@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::RegisterCodec(unsigned long,unsigned long,class IAkSoftwareCodec * __ptr64 (__cdecl*)(void * __ptr64),class IAkSoftwareCodec * __ptr64 (__cdecl*)(void * __ptr64))`
pub const AK_SOUND_ENGINE_REGISTER_CODEC_1EFB379648E2: &CStr = c"?RegisterCodec@SoundEngine@AK@@YA?AW4AKRESULT@@KKP6APEAVIAkSoftwareCodec@@PEAX@Z1@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::RegisterGameObj(unsigned __int64,unsigned long)`
pub const AK_SOUND_ENGINE_REGISTER_GAME_OBJ_4DC65E4B7439: &CStr = c"?RegisterGameObj@SoundEngine@AK@@YA?AW4AKRESULT@@_KK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::RegisterGameObj(unsigned __int64,char const * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_REGISTER_GAME_OBJ_4A05719C2FA3: &CStr = c"?RegisterGameObj@SoundEngine@AK@@YA?AW4AKRESULT@@_KPEBDK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::RegisterGlobalCallback(void (__cdecl*)(bool))`
pub const AK_SOUND_ENGINE_REGISTER_GLOBAL_CALLBACK_CFBE481BC2D0: &CStr = c"?RegisterGlobalCallback@SoundEngine@AK@@YA?AW4AKRESULT@@P6AX_N@Z@Z";

/// code: `void __cdecl AK::MotionEngine::RegisterMotionDevice(unsigned long,unsigned long,class AK::IAkPlugin * __ptr64 (__cdecl*)(class AK::IAkPluginMemAlloc * __ptr64))`
pub const AK_MOTION_ENGINE_REGISTER_MOTION_DEVICE_1997D78B2B8F: &CStr = c"?RegisterMotionDevice@MotionEngine@AK@@YAXKKP6APEAVIAkPlugin@2@PEAVIAkPluginMemAlloc@2@@Z@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::RegisterPlugin(enum AkPluginType,unsigned long,unsigned long,class AK::IAkPlugin * __ptr64 (__cdecl*)(class AK::IAkPluginMemAlloc * __ptr64),class AK::IAkPluginParam * __ptr64 (__cdecl*)(class AK::IAkPluginMemAlloc * __ptr64))`
pub const AK_SOUND_ENGINE_REGISTER_PLUGIN_6423642D6CAD: &CStr = c"?RegisterPlugin@SoundEngine@AK@@YA?AW4AKRESULT@@W4AkPluginType@@KKP6APEAVIAkPlugin@2@PEAVIAkPluginMemAlloc@2@@ZP6APEAVIAkPluginParam@2@1@Z@Z";

/// code: `enum AKRESULT __cdecl AK::MemoryMgr::ReleaseBlock(long,void * __ptr64)`
pub const AK_MEMORY_MGR_RELEASE_BLOCK_F702E837B4B2: &CStr = c"?ReleaseBlock@MemoryMgr@AK@@YA?AW4AKRESULT@@JPEAX@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::RemoveBehavioralExtension(void (__cdecl*)(bool))`
pub const AK_SOUND_ENGINE_REMOVE_BEHAVIORAL_EXTENSION_8022D403C896: &CStr = c"?RemoveBehavioralExtension@SoundEngine@AK@@YA?AW4AKRESULT@@P6AX_N@Z@Z";

/// code: `void __cdecl AK::StreamMgr::RemoveLanguageChangeObserver(void * __ptr64)`
pub const AK_STREAM_MGR_REMOVE_LANGUAGE_CHANGE_OBSERVER_801FDD48FF86: &CStr = c"?RemoveLanguageChangeObserver@StreamMgr@AK@@YAXPEAX@Z";

/// code: `void __cdecl AK::MotionEngine::RemovePlayerMotionDevice(unsigned char,unsigned long,unsigned long)`
pub const AK_MOTION_ENGINE_REMOVE_PLAYER_MOTION_DEVICE_B81CBA4EF93D: &CStr = c"?RemovePlayerMotionDevice@MotionEngine@AK@@YAXEKK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::RemoveSecondaryOutput(unsigned long,enum AkAudioOutputType)`
pub const AK_SOUND_ENGINE_REMOVE_SECONDARY_OUTPUT_CFF9AC4988EE: &CStr = c"?RemoveSecondaryOutput@SoundEngine@AK@@YA?AW4AKRESULT@@KW4AkAudioOutputType@@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::RenderAudio(void)`
pub const AK_SOUND_ENGINE_RENDER_AUDIO_91BFF7E2051C: &CStr = c"?RenderAudio@SoundEngine@AK@@YA?AW4AKRESULT@@XZ";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::ResetRTPCValue(unsigned long,unsigned __int64,long,enum AkCurveInterpolation,bool)`
pub const AK_SOUND_ENGINE_RESET_RTPCVALUE_E3F433512E67: &CStr = c"?ResetRTPCValue@SoundEngine@AK@@YA?AW4AKRESULT@@K_KJW4AkCurveInterpolation@@_N@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::ResetRTPCValue(char const * __ptr64,unsigned __int64,long,enum AkCurveInterpolation,bool)`
pub const AK_SOUND_ENGINE_RESET_RTPCVALUE_9C6535BE07A6: &CStr = c"?ResetRTPCValue@SoundEngine@AK@@YA?AW4AKRESULT@@PEBD_KJW4AkCurveInterpolation@@_N@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::ResetRTPCValue(wchar_t const * __ptr64,unsigned __int64,long,enum AkCurveInterpolation,bool)`
pub const AK_SOUND_ENGINE_RESET_RTPCVALUE_7078AF9E44A6: &CStr = c"?ResetRTPCValue@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_W_KJW4AkCurveInterpolation@@_N@Z";

/// code: `unsigned long __cdecl AK::SoundEngine::DynamicDialogue::ResolveDialogueEvent(unsigned long,unsigned long * __ptr64,unsigned long,unsigned long)`
pub const AK_SOUND_ENGINE_DYNAMIC_DIALOGUE_RESOLVE_DIALOGUE_EVENT_232F85E414C1: &CStr = c"?ResolveDialogueEvent@DynamicDialogue@SoundEngine@AK@@YAKKPEAKKK@Z";

/// code: `unsigned long __cdecl AK::SoundEngine::DynamicDialogue::ResolveDialogueEvent(char const * __ptr64,char const * __ptr64 * __ptr64,unsigned long,unsigned long)`
pub const AK_SOUND_ENGINE_DYNAMIC_DIALOGUE_RESOLVE_DIALOGUE_EVENT_D3B91C015D6E: &CStr = c"?ResolveDialogueEvent@DynamicDialogue@SoundEngine@AK@@YAKPEBDPEAPEBDKK@Z";

/// code: `unsigned long __cdecl AK::SoundEngine::DynamicDialogue::ResolveDialogueEvent(wchar_t const * __ptr64,wchar_t const * __ptr64 * __ptr64,unsigned long,unsigned long)`
pub const AK_SOUND_ENGINE_DYNAMIC_DIALOGUE_RESOLVE_DIALOGUE_EVENT_41F087B67975: &CStr = c"?ResolveDialogueEvent@DynamicDialogue@SoundEngine@AK@@YAKPEB_WPEAPEB_WKK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::DynamicSequence::Resume(unsigned long,long,enum AkCurveInterpolation)`
pub const AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_RESUME_FC1D2EBE625B: &CStr = c"?Resume@DynamicSequence@SoundEngine@AK@@YA?AW4AKRESULT@@KJW4AkCurveInterpolation@@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SeekOnEvent(unsigned long,unsigned __int64,long,bool,unsigned long)`
pub const AK_SOUND_ENGINE_SEEK_ON_EVENT_21A328E1830C: &CStr = c"?SeekOnEvent@SoundEngine@AK@@YA?AW4AKRESULT@@K_KJ_NK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SeekOnEvent(unsigned long,unsigned __int64,float,bool,unsigned long)`
pub const AK_SOUND_ENGINE_SEEK_ON_EVENT_8AFFDA342E41: &CStr = c"?SeekOnEvent@SoundEngine@AK@@YA?AW4AKRESULT@@K_KM_NK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SeekOnEvent(char const * __ptr64,unsigned __int64,long,bool,unsigned long)`
pub const AK_SOUND_ENGINE_SEEK_ON_EVENT_C2767B17720E: &CStr = c"?SeekOnEvent@SoundEngine@AK@@YA?AW4AKRESULT@@PEBD_KJ_NK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SeekOnEvent(char const * __ptr64,unsigned __int64,float,bool,unsigned long)`
pub const AK_SOUND_ENGINE_SEEK_ON_EVENT_64003D93AB84: &CStr = c"?SeekOnEvent@SoundEngine@AK@@YA?AW4AKRESULT@@PEBD_KM_NK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SeekOnEvent(wchar_t const * __ptr64,unsigned __int64,long,bool,unsigned long)`
pub const AK_SOUND_ENGINE_SEEK_ON_EVENT_E6B5112BCFA4: &CStr = c"?SeekOnEvent@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_W_KJ_NK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SeekOnEvent(wchar_t const * __ptr64,unsigned __int64,float,bool,unsigned long)`
pub const AK_SOUND_ENGINE_SEEK_ON_EVENT_71368FB26D8C: &CStr = c"?SeekOnEvent@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_W_KM_NK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetActiveListeners(unsigned __int64,unsigned long)`
pub const AK_SOUND_ENGINE_SET_ACTIVE_LISTENERS_E633F2B818D5: &CStr = c"?SetActiveListeners@SoundEngine@AK@@YA?AW4AKRESULT@@_KK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetActorMixerEffect(unsigned long,unsigned long,unsigned long)`
pub const AK_SOUND_ENGINE_SET_ACTOR_MIXER_EFFECT_824862EBE2F1: &CStr = c"?SetActorMixerEffect@SoundEngine@AK@@YA?AW4AKRESULT@@KKK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetAttenuationScalingFactor(unsigned __int64,float)`
pub const AK_SOUND_ENGINE_SET_ATTENUATION_SCALING_FACTOR_637652D784EA: &CStr = c"?SetAttenuationScalingFactor@SoundEngine@AK@@YA?AW4AKRESULT@@_KM@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetBankLoadIOSettings(float,char)`
pub const AK_SOUND_ENGINE_SET_BANK_LOAD_IOSETTINGS_E6A35F8EFF8F: &CStr = c"?SetBankLoadIOSettings@SoundEngine@AK@@YA?AW4AKRESULT@@MD@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetBusEffect(unsigned long,unsigned long,unsigned long)`
pub const AK_SOUND_ENGINE_SET_BUS_EFFECT_62B1F51052FC: &CStr = c"?SetBusEffect@SoundEngine@AK@@YA?AW4AKRESULT@@KKK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetBusEffect(char const * __ptr64,unsigned long,unsigned long)`
pub const AK_SOUND_ENGINE_SET_BUS_EFFECT_83341329B635: &CStr = c"?SetBusEffect@SoundEngine@AK@@YA?AW4AKRESULT@@PEBDKK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetBusEffect(wchar_t const * __ptr64,unsigned long,unsigned long)`
pub const AK_SOUND_ENGINE_SET_BUS_EFFECT_74AD1A581C3B: &CStr = c"?SetBusEffect@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_WKK@Z";

/// code: `enum AKRESULT __cdecl AK::StreamMgr::SetCurrentLanguage(wchar_t const * __ptr64)`
pub const AK_STREAM_MGR_SET_CURRENT_LANGUAGE_D4905698BF16: &CStr = c"?SetCurrentLanguage@StreamMgr@AK@@YA?AW4AKRESULT@@PEB_W@Z";

/// code: `void __cdecl AK::StreamMgr::SetFileLocationResolver(class AK::StreamMgr::IAkFileLocationResolver * __ptr64)`
pub const AK_STREAM_MGR_SET_FILE_LOCATION_RESOLVER_AA9F03D2CB3D: &CStr = c"?SetFileLocationResolver@StreamMgr@AK@@YAXPEAVIAkFileLocationResolver@12@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetGameObjectAuxSendValues(unsigned __int64,struct AkAuxSendValue * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_SET_GAME_OBJECT_AUX_SEND_VALUES_40EA09999A88: &CStr = c"?SetGameObjectAuxSendValues@SoundEngine@AK@@YA?AW4AKRESULT@@_KPEAUAkAuxSendValue@@K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetGameObjectOutputBusVolume(unsigned __int64,float)`
pub const AK_SOUND_ENGINE_SET_GAME_OBJECT_OUTPUT_BUS_VOLUME_8C7DAEF7FDB6: &CStr = c"?SetGameObjectOutputBusVolume@SoundEngine@AK@@YA?AW4AKRESULT@@_KM@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetListenerPipeline(unsigned long,bool,bool)`
pub const AK_SOUND_ENGINE_SET_LISTENER_PIPELINE_E898C9FE9FAE: &CStr = c"?SetListenerPipeline@SoundEngine@AK@@YA?AW4AKRESULT@@K_N0@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetListenerPosition(struct AkListenerPosition const & __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_SET_LISTENER_POSITION_5E7C0593BD0B: &CStr = c"?SetListenerPosition@SoundEngine@AK@@YA?AW4AKRESULT@@AEBUAkListenerPosition@@K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetListenerScalingFactor(unsigned long,float)`
pub const AK_SOUND_ENGINE_SET_LISTENER_SCALING_FACTOR_F6D9B99A3930: &CStr = c"?SetListenerScalingFactor@SoundEngine@AK@@YA?AW4AKRESULT@@KM@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetListenerSpatialization(unsigned long,bool,struct AkChannelConfig,float * __ptr64)`
pub const AK_SOUND_ENGINE_SET_LISTENER_SPATIALIZATION_F3E38FB62DBE: &CStr = c"?SetListenerSpatialization@SoundEngine@AK@@YA?AW4AKRESULT@@K_NUAkChannelConfig@@PEAM@Z";

/// code: `enum AKRESULT __cdecl AK::Monitor::SetLocalOutput(unsigned long,void (__cdecl*)(enum AK::Monitor::ErrorCode,wchar_t const * __ptr64,enum AK::Monitor::ErrorLevel,unsigned long,unsigned __int64))`
pub const AK_MONITOR_SET_LOCAL_OUTPUT_7088FF58E8E6: &CStr = c"?SetLocalOutput@Monitor@AK@@YA?AW4AKRESULT@@KP6AXW4ErrorCode@12@PEB_WW4ErrorLevel@12@K_K@Z@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetMaxNumVoicesLimit(unsigned short)`
pub const AK_SOUND_ENGINE_SET_MAX_NUM_VOICES_LIMIT_FDB0FB30B8B5: &CStr = c"?SetMaxNumVoicesLimit@SoundEngine@AK@@YA?AW4AKRESULT@@G@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetMedia(struct AkSourceSettings * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_SET_MEDIA_9E1BDFFAF79A: &CStr = c"?SetMedia@SoundEngine@AK@@YA?AW4AKRESULT@@PEAUAkSourceSettings@@K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetMixer(unsigned long,unsigned long)`
pub const AK_SOUND_ENGINE_SET_MIXER_65E7BC4C183E: &CStr = c"?SetMixer@SoundEngine@AK@@YA?AW4AKRESULT@@KK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetMixer(char const * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_SET_MIXER_C05DF64D808B: &CStr = c"?SetMixer@SoundEngine@AK@@YA?AW4AKRESULT@@PEBDK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetMixer(wchar_t const * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_SET_MIXER_D2D84390AC7E: &CStr = c"?SetMixer@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_WK@Z";

/// code: `enum AKRESULT __cdecl AK::MemoryMgr::SetMonitoring(long,bool)`
pub const AK_MEMORY_MGR_SET_MONITORING_9C2F4379A783: &CStr = c"?SetMonitoring@MemoryMgr@AK@@YA?AW4AKRESULT@@J_N@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetMultiplePositions(unsigned __int64,struct AkSoundPosition const * __ptr64,unsigned short,enum AK::SoundEngine::MultiPositionType)`
pub const AK_SOUND_ENGINE_SET_MULTIPLE_POSITIONS_C21E5A42F749: &CStr = c"?SetMultiplePositions@SoundEngine@AK@@YA?AW4AKRESULT@@_KPEBUAkSoundPosition@@GW4MultiPositionType@12@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetObjectObstructionAndOcclusion(unsigned __int64,unsigned long,float,float)`
pub const AK_SOUND_ENGINE_SET_OBJECT_OBSTRUCTION_AND_OCCLUSION_98B692FBFDA8: &CStr = c"?SetObjectObstructionAndOcclusion@SoundEngine@AK@@YA?AW4AKRESULT@@_KKMM@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetPanningRule(enum AkPanningRule,enum AkAudioOutputType,unsigned long)`
pub const AK_SOUND_ENGINE_SET_PANNING_RULE_E9F648015123: &CStr = c"?SetPanningRule@SoundEngine@AK@@YA?AW4AKRESULT@@W4AkPanningRule@@W4AkAudioOutputType@@K@Z";

/// code: `void __cdecl AK::MotionEngine::SetPlayerListener(unsigned char,unsigned char)`
pub const AK_MOTION_ENGINE_SET_PLAYER_LISTENER_401393052625: &CStr = c"?SetPlayerListener@MotionEngine@AK@@YAXEE@Z";

/// code: `void __cdecl AK::MotionEngine::SetPlayerVolume(unsigned char,float)`
pub const AK_MOTION_ENGINE_SET_PLAYER_VOLUME_0E836955CCE2: &CStr = c"?SetPlayerVolume@MotionEngine@AK@@YAXEM@Z";

/// code: `enum AKRESULT __cdecl AK::MemoryMgr::SetPoolName(long,char const * __ptr64)`
pub const AK_MEMORY_MGR_SET_POOL_NAME_9F4D84CE545D: &CStr = c"?SetPoolName@MemoryMgr@AK@@YA?AW4AKRESULT@@JPEBD@Z";

/// code: `enum AKRESULT __cdecl AK::MemoryMgr::SetPoolName(long,wchar_t const * __ptr64)`
pub const AK_MEMORY_MGR_SET_POOL_NAME_EEF0933C4318: &CStr = c"?SetPoolName@MemoryMgr@AK@@YA?AW4AKRESULT@@JPEB_W@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetPosition(unsigned __int64,struct AkSoundPosition const & __ptr64)`
pub const AK_SOUND_ENGINE_SET_POSITION_6AF8247AFD54: &CStr = c"?SetPosition@SoundEngine@AK@@YA?AW4AKRESULT@@_KAEBUAkSoundPosition@@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetPositionInternal(unsigned __int64,struct AkSoundPosition const & __ptr64)`
pub const AK_SOUND_ENGINE_SET_POSITION_INTERNAL_F2FAC3C13E59: &CStr = c"?SetPositionInternal@SoundEngine@AK@@YA?AW4AKRESULT@@_KAEBUAkSoundPosition@@@Z";

/// code: `public: enum AKRESULT __cdecl CAkRTPCMgr::SetRTPCInternal(unsigned long,float,class AkRTPCKey const & __ptr64,struct TransParamsBase & __ptr64,enum AkValueMeaning) __ptr64`
pub const CAK_RTPCMGR_SET_RTPCINTERNAL_414ED05CD68C: &CStr = c"?SetRTPCInternal@CAkRTPCMgr@@QEAA?AW4AKRESULT@@KMAEBVAkRTPCKey@@AEAUTransParamsBase@@W4AkValueMeaning@@@Z";

/// code: `public: enum AKRESULT __cdecl CAkRTPCMgr::SetRTPCInternal(unsigned long,float,class CAkRegisteredObj * __ptr64,unsigned long,struct TransParamsBase & __ptr64,enum AkValueMeaning) __ptr64`
pub const CAK_RTPCMGR_SET_RTPCINTERNAL_DFFBB40FA663: &CStr = c"?SetRTPCInternal@CAkRTPCMgr@@QEAA?AW4AKRESULT@@KMPEAVCAkRegisteredObj@@KAEAUTransParamsBase@@W4AkValueMeaning@@@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetRTPCValue(unsigned long,float,unsigned __int64,long,enum AkCurveInterpolation,bool)`
pub const AK_SOUND_ENGINE_SET_RTPCVALUE_224B8C27AD2B: &CStr = c"?SetRTPCValue@SoundEngine@AK@@YA?AW4AKRESULT@@KM_KJW4AkCurveInterpolation@@_N@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetRTPCValue(char const * __ptr64,float,unsigned __int64,long,enum AkCurveInterpolation,bool)`
pub const AK_SOUND_ENGINE_SET_RTPCVALUE_B50C10EA0A1A: &CStr = c"?SetRTPCValue@SoundEngine@AK@@YA?AW4AKRESULT@@PEBDM_KJW4AkCurveInterpolation@@_N@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetRTPCValue(wchar_t const * __ptr64,float,unsigned __int64,long,enum AkCurveInterpolation,bool)`
pub const AK_SOUND_ENGINE_SET_RTPCVALUE_67FEF9D2AC6E: &CStr = c"?SetRTPCValue@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_WM_KJW4AkCurveInterpolation@@_N@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetRTPCValueByPlayingID(unsigned long,float,unsigned long,long,enum AkCurveInterpolation,bool)`
pub const AK_SOUND_ENGINE_SET_RTPCVALUE_BY_PLAYING_ID_BE7FFD7323AC: &CStr = c"?SetRTPCValueByPlayingID@SoundEngine@AK@@YA?AW4AKRESULT@@KMKJW4AkCurveInterpolation@@_N@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetRTPCValueByPlayingID(char const * __ptr64,float,unsigned long,long,enum AkCurveInterpolation,bool)`
pub const AK_SOUND_ENGINE_SET_RTPCVALUE_BY_PLAYING_ID_731F7B55D939: &CStr = c"?SetRTPCValueByPlayingID@SoundEngine@AK@@YA?AW4AKRESULT@@PEBDMKJW4AkCurveInterpolation@@_N@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetRTPCValueByPlayingID(wchar_t const * __ptr64,float,unsigned long,long,enum AkCurveInterpolation,bool)`
pub const AK_SOUND_ENGINE_SET_RTPCVALUE_BY_PLAYING_ID_AC404947BBF4: &CStr = c"?SetRTPCValueByPlayingID@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_WMKJW4AkCurveInterpolation@@_N@Z";

/// code: `void __cdecl AK::SoundEngine::SetRandomSeed(unsigned long)`
pub const AK_SOUND_ENGINE_SET_RANDOM_SEED_21A287AADD5E: &CStr = c"?SetRandomSeed@SoundEngine@AK@@YAXK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetSecondaryOutputVolume(unsigned long,enum AkAudioOutputType,float)`
pub const AK_SOUND_ENGINE_SET_SECONDARY_OUTPUT_VOLUME_A120B97E25C9: &CStr = c"?SetSecondaryOutputVolume@SoundEngine@AK@@YA?AW4AKRESULT@@KW4AkAudioOutputType@@M@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetSpeakerAngles(float * __ptr64,unsigned long,float,enum AkAudioOutputType,unsigned long)`
pub const AK_SOUND_ENGINE_SET_SPEAKER_ANGLES_5392B7255EF6: &CStr = c"?SetSpeakerAngles@SoundEngine@AK@@YA?AW4AKRESULT@@PEAMKMW4AkAudioOutputType@@K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetState(unsigned long,unsigned long)`
pub const AK_SOUND_ENGINE_SET_STATE_AFACB292FBF1: &CStr = c"?SetState@SoundEngine@AK@@YA?AW4AKRESULT@@KK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetState(char const * __ptr64,char const * __ptr64)`
pub const AK_SOUND_ENGINE_SET_STATE_128B82BA0EB3: &CStr = c"?SetState@SoundEngine@AK@@YA?AW4AKRESULT@@PEBD0@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetState(wchar_t const * __ptr64,wchar_t const * __ptr64)`
pub const AK_SOUND_ENGINE_SET_STATE_B3B1172B53D3: &CStr = c"?SetState@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_W0@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetSwitch(unsigned long,unsigned long,unsigned __int64)`
pub const AK_SOUND_ENGINE_SET_SWITCH_02C7DECC43D8: &CStr = c"?SetSwitch@SoundEngine@AK@@YA?AW4AKRESULT@@KK_K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetSwitch(char const * __ptr64,char const * __ptr64,unsigned __int64)`
pub const AK_SOUND_ENGINE_SET_SWITCH_F7F4F12BF736: &CStr = c"?SetSwitch@SoundEngine@AK@@YA?AW4AKRESULT@@PEBD0_K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetSwitch(wchar_t const * __ptr64,wchar_t const * __ptr64,unsigned __int64)`
pub const AK_SOUND_ENGINE_SET_SWITCH_6D72557ADAD9: &CStr = c"?SetSwitch@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_W0_K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::SetVolumeThreshold(float)`
pub const AK_SOUND_ENGINE_SET_VOLUME_THRESHOLD_0BCAD0D7D4D1: &CStr = c"?SetVolumeThreshold@SoundEngine@AK@@YA?AW4AKRESULT@@M@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::StartOutputCapture(wchar_t const * __ptr64)`
pub const AK_SOUND_ENGINE_START_OUTPUT_CAPTURE_FC6370DC5B8D: &CStr = c"?StartOutputCapture@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_W@Z";

/// code: `void __cdecl AK::MemoryMgr::StartProfileThreadUsage(long)`
pub const AK_MEMORY_MGR_START_PROFILE_THREAD_USAGE_AF272D6CC4FA: &CStr = c"?StartProfileThreadUsage@MemoryMgr@AK@@YAXJ@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::StartProfilerCapture(wchar_t const * __ptr64)`
pub const AK_SOUND_ENGINE_START_PROFILER_CAPTURE_6DAB11C93B4C: &CStr = c"?StartProfilerCapture@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_W@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::DynamicSequence::Stop(unsigned long,long,enum AkCurveInterpolation)`
pub const AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_STOP_3CEF204C8D2E: &CStr = c"?Stop@DynamicSequence@SoundEngine@AK@@YA?AW4AKRESULT@@KJW4AkCurveInterpolation@@@Z";

/// code: `void __cdecl AK::SoundEngine::StopAll(unsigned __int64)`
pub const AK_SOUND_ENGINE_STOP_ALL_C67520D79749: &CStr = c"?StopAll@SoundEngine@AK@@YAX_K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::StopOutputCapture(void)`
pub const AK_SOUND_ENGINE_STOP_OUTPUT_CAPTURE_8865D78163FC: &CStr = c"?StopOutputCapture@SoundEngine@AK@@YA?AW4AKRESULT@@XZ";

/// code: `void __cdecl AK::SoundEngine::StopPlayingID(unsigned long,long,enum AkCurveInterpolation)`
pub const AK_SOUND_ENGINE_STOP_PLAYING_ID_187315448957: &CStr = c"?StopPlayingID@SoundEngine@AK@@YAXKJW4AkCurveInterpolation@@@Z";

/// code: `unsigned long __cdecl AK::MemoryMgr::StopProfileThreadUsage(long)`
pub const AK_MEMORY_MGR_STOP_PROFILE_THREAD_USAGE_C6862BD685B1: &CStr = c"?StopProfileThreadUsage@MemoryMgr@AK@@YAKJ@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::StopProfilerCapture(void)`
pub const AK_SOUND_ENGINE_STOP_PROFILER_CAPTURE_EBF843AE6A95: &CStr = c"?StopProfilerCapture@SoundEngine@AK@@YA?AW4AKRESULT@@XZ";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::StopSourcePlugin(unsigned long,unsigned long,unsigned long)`
pub const AK_SOUND_ENGINE_STOP_SOURCE_PLUGIN_B44BDF831850: &CStr = c"?StopSourcePlugin@SoundEngine@AK@@YA?AW4AKRESULT@@KKK@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::Suspend(bool)`
pub const AK_SOUND_ENGINE_SUSPEND_44AF7646D030: &CStr = c"?Suspend@SoundEngine@AK@@YA?AW4AKRESULT@@_N@Z";

/// code: `void __cdecl AK::MemoryMgr::Term(void)`
pub const AK_MEMORY_MGR_TERM_139C019C551C: &CStr = c"?Term@MemoryMgr@AK@@YAXXZ";

/// code: `void __cdecl AK::MusicEngine::Term(void)`
pub const AK_MUSIC_ENGINE_TERM_DDAC00E33B2D: &CStr = c"?Term@MusicEngine@AK@@YAXXZ";

/// code: `void __cdecl AK::SoundEngine::Term(void)`
pub const AK_SOUND_ENGINE_TERM_2DB29D9832F3: &CStr = c"?Term@SoundEngine@AK@@YAXXZ";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::UnloadBank(unsigned long,void const * __ptr64,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64)`
pub const AK_SOUND_ENGINE_UNLOAD_BANK_337F97155ED4: &CStr = c"?UnloadBank@SoundEngine@AK@@YA?AW4AKRESULT@@KPEBXP6AXK0W43@JPEAX@Z2@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::UnloadBank(unsigned long,void const * __ptr64,long * __ptr64)`
pub const AK_SOUND_ENGINE_UNLOAD_BANK_9655D2964E83: &CStr = c"?UnloadBank@SoundEngine@AK@@YA?AW4AKRESULT@@KPEBXPEAJ@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::UnloadBank(char const * __ptr64,void const * __ptr64,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64)`
pub const AK_SOUND_ENGINE_UNLOAD_BANK_67235594F14F: &CStr = c"?UnloadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEBDPEBXP6AXK1W43@JPEAX@Z3@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::UnloadBank(char const * __ptr64,void const * __ptr64,long * __ptr64)`
pub const AK_SOUND_ENGINE_UNLOAD_BANK_54C19FD1F2E0: &CStr = c"?UnloadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEBDPEBXPEAJ@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::UnloadBank(wchar_t const * __ptr64,void const * __ptr64,void (__cdecl*)(unsigned long,void const * __ptr64,enum AKRESULT,long,void * __ptr64),void * __ptr64)`
pub const AK_SOUND_ENGINE_UNLOAD_BANK_F45EED2F24D3: &CStr = c"?UnloadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_WPEBXP6AXK1W43@JPEAX@Z3@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::UnloadBank(wchar_t const * __ptr64,void const * __ptr64,long * __ptr64)`
pub const AK_SOUND_ENGINE_UNLOAD_BANK_59B0956756B0: &CStr = c"?UnloadBank@SoundEngine@AK@@YA?AW4AKRESULT@@PEB_WPEBXPEAJ@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::DynamicSequence::UnlockPlaylist(unsigned long)`
pub const AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_UNLOCK_PLAYLIST_A2AE8B6FF790: &CStr = c"?UnlockPlaylist@DynamicSequence@SoundEngine@AK@@YA?AW4AKRESULT@@K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::UnregisterAllGameObj(void)`
pub const AK_SOUND_ENGINE_UNREGISTER_ALL_GAME_OBJ_A57234B33D8B: &CStr = c"?UnregisterAllGameObj@SoundEngine@AK@@YA?AW4AKRESULT@@XZ";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::UnregisterGameObj(unsigned __int64)`
pub const AK_SOUND_ENGINE_UNREGISTER_GAME_OBJ_0468761A228E: &CStr = c"?UnregisterGameObj@SoundEngine@AK@@YA?AW4AKRESULT@@_K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::UnregisterGlobalCallback(void (__cdecl*)(bool))`
pub const AK_SOUND_ENGINE_UNREGISTER_GLOBAL_CALLBACK_9B4E4EBF2505: &CStr = c"?UnregisterGlobalCallback@SoundEngine@AK@@YA?AW4AKRESULT@@P6AX_N@Z@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::UnsetMedia(struct AkSourceSettings * __ptr64,unsigned long)`
pub const AK_SOUND_ENGINE_UNSET_MEDIA_74AE9F3BE0AF: &CStr = c"?UnsetMedia@SoundEngine@AK@@YA?AW4AKRESULT@@PEAUAkSourceSettings@@K@Z";

/// code: `enum AKRESULT __cdecl AK::SoundEngine::WakeupFromSuspend(void)`
pub const AK_SOUND_ENGINE_WAKEUP_FROM_SUSPEND_0EE38A533A98: &CStr = c"?WakeupFromSuspend@SoundEngine@AK@@YA?AW4AKRESULT@@XZ";

/// data: `long g_DefaultPoolId`
pub const G_DEFAULT_POOL_ID_FC6F8C366D59: &CStr = c"?g_DefaultPoolId@@3JA";

/// data: `long g_LEngineDefaultPoolId`
pub const G_LENGINE_DEFAULT_POOL_ID_C2010F2CDC74: &CStr = c"?g_LEngineDefaultPoolId@@3JA";

/// data: `unsigned long AK::SoundEngine::g_PlayingID`
pub const AK_SOUND_ENGINE_G_PLAYING_ID_2B5E59609F6B: &CStr = c"?g_PlayingID@SoundEngine@AK@@3KA";

/// data: `void (__cdecl* __ptr64 g_pAssertHook)(char const * __ptr64,char const * __ptr64,int)`
pub const G_P_ASSERT_HOOK_B24C36F35C65: &CStr = c"?g_pAssertHook@@3P6AXPEBD0H@ZEA";

/// data: `class CAkRTPCMgr * __ptr64 __ptr64 g_pRTPCMgr`
pub const G_P_RTPCMGR_B8D81F181C0D: &CStr = c"?g_pRTPCMgr@@3PEAVCAkRTPCMgr@@EA";

/// data: `class CAkRegistryMgr * __ptr64 __ptr64 g_pRegistryMgr`
pub const G_P_REGISTRY_MGR_A5AE18348471: &CStr = c"?g_pRegistryMgr@@3PEAVCAkRegistryMgr@@EA";

/// data: `class CAkSwitchMgr * __ptr64 __ptr64 g_pSwitchMgr`
pub const G_P_SWITCH_MGR_CA741FFE2530: &CStr = c"?g_pSwitchMgr@@3PEAVCAkSwitchMgr@@EA";

/// data: `protected: static class AK::IAkStreamMgr * __ptr64 __ptr64 AK::IAkStreamMgr::m_pStreamMgr`
pub const AK_IAK_STREAM_MGR_M_P_STREAM_MGR_AB66837AE9D3: &CStr = c"?m_pStreamMgr@IAkStreamMgr@AK@@1PEAV12@EA";

/// data: `NvOptimusEnablement`
pub const NV_OPTIMUS_ENABLEMENT_B6ED52D21305: &CStr = c"NvOptimusEnablement";

/// All named exports, including data and forwarders; not a list of callable hooks.
pub const EXPORTED_NAMES: &[&CStr] = &[
    CAK_REGISTERED_OBJ_DESTROY_E5A7CACB1F49,
    AK_SOUND_ENGINE_ADD_BEHAVIORAL_EXTENSION_9D90E1043CF6,
    AK_STREAM_MGR_ADD_LANGUAGE_CHANGE_OBSERVER_AB1EA59D4F71,
    AK_SOUND_ENGINE_ADD_OUTPUT_CAPTURE_MARKER_B222514CE730,
    AK_MOTION_ENGINE_ADD_PLAYER_MOTION_DEVICE_8C38E92E1846,
    AK_SOUND_ENGINE_ADD_SECONDARY_OUTPUT_0681313D0C6B,
    AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_BREAK_22FC59F23D9B,
    AK_SOUND_ENGINE_CANCEL_BANK_CALLBACK_COOKIE_54ADF0EA1A24,
    AK_SOUND_ENGINE_CANCEL_EVENT_CALLBACK_B4C546A9D8B6,
    AK_SOUND_ENGINE_CANCEL_EVENT_CALLBACK_COOKIE_7164FF264B70,
    AK_MEMORY_MGR_CHECK_POOL_ID_18C7816585B0,
    AK_SOUND_ENGINE_CLEAR_BANKS_41986F9F4ECE,
    AK_SOUND_ENGINE_CLEAR_PREPARED_EVENTS_15ED182A03AE,
    AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_CLOSE_AB846A881AC3,
    AK_STREAM_MGR_CREATE_F996534A33FC,
    AK_STREAM_MGR_CREATE_DEVICE_FB2486DE0368,
    AK_MEMORY_MGR_CREATE_POOL_18CA554CEB75,
    AK_STREAM_MGR_DESTROY_DEVICE_93CA085272CF,
    AK_MEMORY_MGR_DESTROY_POOL_D2CEDFDFAE6A,
    AK_SOUND_ENGINE_EXECUTE_ACTION_ON_EVENT_CFF0C23D5775,
    AK_SOUND_ENGINE_EXECUTE_ACTION_ON_EVENT_BC4592C854DC,
    AK_SOUND_ENGINE_EXECUTE_ACTION_ON_EVENT_6E34E4AC5DFA,
    AK_MEMORY_MGR_FALIGN_A3C720AD9801,
    AK_STREAM_MGR_FLUSH_ALL_CACHES_4BF1A9544EB2,
    AK_MEMORY_MGR_FREE_A86D584F2BF9,
    AK_MEMORY_MGR_GET_BLOCK_A253FF7FAE5F,
    AK_MEMORY_MGR_GET_BLOCK_SIZE_C6483D487C16,
    AK_STREAM_MGR_GET_CURRENT_LANGUAGE_FBBF37CFBCF1,
    AK_STREAM_MGR_GET_DEFAULT_DEVICE_SETTINGS_C165EA988679,
    AK_MUSIC_ENGINE_GET_DEFAULT_INIT_SETTINGS_E232787366BF,
    AK_SOUND_ENGINE_GET_DEFAULT_INIT_SETTINGS_1276F3B87CB6,
    AK_SOUND_ENGINE_GET_DEFAULT_PLATFORM_INIT_SETTINGS_BC90DCDE222D,
    AK_STREAM_MGR_GET_DEFAULT_SETTINGS_A4B33701A6B3,
    AK_GET_DIRECT_SOUND_INSTANCE_A0C28141EC61,
    AK_STREAM_MGR_GET_FILE_LOCATION_RESOLVER_0DD219FCC4E9,
    AK_SOUND_ENGINE_GET_IDFROM_STRING_508DCB26719F,
    AK_SOUND_ENGINE_GET_IDFROM_STRING_69A82739C07B,
    AK_MEMORY_MGR_GET_MAX_POOLS_865D9EEDACE0,
    AK_MEMORY_MGR_GET_NUM_POOLS_B63C9E35D262,
    CAK_REGISTRY_MGR_GET_OBJ_AND_ADDREF_4C20D18EE446,
    AK_SOUND_ENGINE_GET_PANNING_RULE_7578D99D51E1,
    AK_MUSIC_ENGINE_GET_PLAYING_SEGMENT_INFO_9F952566AF7C,
    AK_MEMORY_MGR_GET_POOL_ATTRIBUTES_D599CDDA0B18,
    AK_STREAM_MGR_GET_POOL_ID_0E1D100CC585,
    AK_MEMORY_MGR_GET_POOL_MEMORY_USED_17407D3262F3,
    AK_MEMORY_MGR_GET_POOL_NAME_A42C13004EF9,
    AK_MEMORY_MGR_GET_POOL_STATS_87DDF30725DC,
    AK_SOUND_ENGINE_GET_SOURCE_PLAY_POSITION_4D5BABCF29B9,
    AK_SOUND_ENGINE_GET_SOURCE_STREAM_BUFFERING_7C3266A34862,
    AK_SOUND_ENGINE_GET_SPEAKER_ANGLES_A0BFCAB16944,
    AK_SOUND_ENGINE_GET_SPEAKER_CONFIGURATION_92856BED21DA,
    AK_MONITOR_GET_TIME_STAMP_76911C27D7C7,
    AK_GET_WWISE_XAUDIO2_INTERFACE_3BC5E1D5B37B,
    AK_MUSIC_ENGINE_INIT_FCD348A45990,
    AK_SOUND_ENGINE_INIT_0D2F1CAAA52B,
    AK_MEMORY_MGR_IS_INITIALIZED_1BEECB520943,
    AK_SOUND_ENGINE_IS_INITIALIZED_1E2CA3D17A16,
    AK_SOUND_ENGINE_LOAD_BANK_29A29AAB4F6D,
    AK_SOUND_ENGINE_LOAD_BANK_2FAE5ABAB905,
    AK_SOUND_ENGINE_LOAD_BANK_AD65959920C3,
    AK_SOUND_ENGINE_LOAD_BANK_6FFB2BE7212A,
    AK_SOUND_ENGINE_LOAD_BANK_1324DD9473F0,
    AK_SOUND_ENGINE_LOAD_BANK_9B1E1B7BCF00,
    AK_SOUND_ENGINE_LOAD_BANK_88EBC790AC04,
    AK_SOUND_ENGINE_LOAD_BANK_509CF55D157D,
    AK_SOUND_ENGINE_LOAD_BANK_7B66A39B71FE,
    AK_SOUND_ENGINE_LOAD_BANK_7151896015F4,
    AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_LOCK_PLAYLIST_5337987BA7E1,
    AK_MEMORY_MGR_MALIGN_AC3524AEA0A2,
    AK_MEMORY_MGR_MALLOC_DB1E34830BA8,
    AK_SOUND_ENGINE_MUTE_BACKGROUND_MUSIC_917D3D285058,
    AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_OPEN_47F58EE88370,
    AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_PAUSE_46B932BAD62B,
    AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_PLAY_191E761E4A09,
    AK_SOUND_ENGINE_PLAY_SOURCE_PLUGIN_D8322C98F830,
    AK_MONITOR_POST_CODE_5E81F0658BC7,
    AK_SOUND_ENGINE_POST_EVENT_9C7636B50778,
    AK_SOUND_ENGINE_POST_EVENT_0EA4B18D2597,
    AK_SOUND_ENGINE_POST_EVENT_D8C0ED3B4722,
    AK_MONITOR_POST_STRING_19871C7F5148,
    AK_MONITOR_POST_STRING_A5A143AB7935,
    AK_SOUND_ENGINE_POST_TRIGGER_54E7EA3ED4DB,
    AK_SOUND_ENGINE_POST_TRIGGER_1B80238DC1E5,
    AK_SOUND_ENGINE_POST_TRIGGER_1BD23F19E5B0,
    AK_SOUND_ENGINE_PREPARE_BANK_7009801B3A20,
    AK_SOUND_ENGINE_PREPARE_BANK_75145F2DB484,
    AK_SOUND_ENGINE_PREPARE_BANK_B418D4A8520A,
    AK_SOUND_ENGINE_PREPARE_BANK_DD1553532714,
    AK_SOUND_ENGINE_PREPARE_BANK_8C669803C604,
    AK_SOUND_ENGINE_PREPARE_BANK_FA0B3C43DD2D,
    AK_SOUND_ENGINE_PREPARE_EVENT_7618CB54A057,
    AK_SOUND_ENGINE_PREPARE_EVENT_9EB838DF9A50,
    AK_SOUND_ENGINE_PREPARE_EVENT_896E426295F8,
    AK_SOUND_ENGINE_PREPARE_EVENT_56A420F01080,
    AK_SOUND_ENGINE_PREPARE_EVENT_56C05AADF115,
    AK_SOUND_ENGINE_PREPARE_EVENT_662614AE28AA,
    AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_C0F882472841,
    AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_1B876E474B24,
    AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_ED73BA38926C,
    AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_528043A8B362,
    AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_1B5F3FAEBBF6,
    AK_SOUND_ENGINE_PREPARE_GAME_SYNCS_31A0FC352D2C,
    AK_MONITOR_DATA_REAL_SIZEOF_8E3F0E572D6F,
    AK_SOUND_ENGINE_REGISTER_BUS_METERING_CALLBACK_6D77D3742B8D,
    AK_SOUND_ENGINE_REGISTER_BUS_VOLUME_CALLBACK_C43FF1BB9E63,
    AK_SOUND_ENGINE_REGISTER_CODEC_1EFB379648E2,
    AK_SOUND_ENGINE_REGISTER_GAME_OBJ_4DC65E4B7439,
    AK_SOUND_ENGINE_REGISTER_GAME_OBJ_4A05719C2FA3,
    AK_SOUND_ENGINE_REGISTER_GLOBAL_CALLBACK_CFBE481BC2D0,
    AK_MOTION_ENGINE_REGISTER_MOTION_DEVICE_1997D78B2B8F,
    AK_SOUND_ENGINE_REGISTER_PLUGIN_6423642D6CAD,
    AK_MEMORY_MGR_RELEASE_BLOCK_F702E837B4B2,
    AK_SOUND_ENGINE_REMOVE_BEHAVIORAL_EXTENSION_8022D403C896,
    AK_STREAM_MGR_REMOVE_LANGUAGE_CHANGE_OBSERVER_801FDD48FF86,
    AK_MOTION_ENGINE_REMOVE_PLAYER_MOTION_DEVICE_B81CBA4EF93D,
    AK_SOUND_ENGINE_REMOVE_SECONDARY_OUTPUT_CFF9AC4988EE,
    AK_SOUND_ENGINE_RENDER_AUDIO_91BFF7E2051C,
    AK_SOUND_ENGINE_RESET_RTPCVALUE_E3F433512E67,
    AK_SOUND_ENGINE_RESET_RTPCVALUE_9C6535BE07A6,
    AK_SOUND_ENGINE_RESET_RTPCVALUE_7078AF9E44A6,
    AK_SOUND_ENGINE_DYNAMIC_DIALOGUE_RESOLVE_DIALOGUE_EVENT_232F85E414C1,
    AK_SOUND_ENGINE_DYNAMIC_DIALOGUE_RESOLVE_DIALOGUE_EVENT_D3B91C015D6E,
    AK_SOUND_ENGINE_DYNAMIC_DIALOGUE_RESOLVE_DIALOGUE_EVENT_41F087B67975,
    AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_RESUME_FC1D2EBE625B,
    AK_SOUND_ENGINE_SEEK_ON_EVENT_21A328E1830C,
    AK_SOUND_ENGINE_SEEK_ON_EVENT_8AFFDA342E41,
    AK_SOUND_ENGINE_SEEK_ON_EVENT_C2767B17720E,
    AK_SOUND_ENGINE_SEEK_ON_EVENT_64003D93AB84,
    AK_SOUND_ENGINE_SEEK_ON_EVENT_E6B5112BCFA4,
    AK_SOUND_ENGINE_SEEK_ON_EVENT_71368FB26D8C,
    AK_SOUND_ENGINE_SET_ACTIVE_LISTENERS_E633F2B818D5,
    AK_SOUND_ENGINE_SET_ACTOR_MIXER_EFFECT_824862EBE2F1,
    AK_SOUND_ENGINE_SET_ATTENUATION_SCALING_FACTOR_637652D784EA,
    AK_SOUND_ENGINE_SET_BANK_LOAD_IOSETTINGS_E6A35F8EFF8F,
    AK_SOUND_ENGINE_SET_BUS_EFFECT_62B1F51052FC,
    AK_SOUND_ENGINE_SET_BUS_EFFECT_83341329B635,
    AK_SOUND_ENGINE_SET_BUS_EFFECT_74AD1A581C3B,
    AK_STREAM_MGR_SET_CURRENT_LANGUAGE_D4905698BF16,
    AK_STREAM_MGR_SET_FILE_LOCATION_RESOLVER_AA9F03D2CB3D,
    AK_SOUND_ENGINE_SET_GAME_OBJECT_AUX_SEND_VALUES_40EA09999A88,
    AK_SOUND_ENGINE_SET_GAME_OBJECT_OUTPUT_BUS_VOLUME_8C7DAEF7FDB6,
    AK_SOUND_ENGINE_SET_LISTENER_PIPELINE_E898C9FE9FAE,
    AK_SOUND_ENGINE_SET_LISTENER_POSITION_5E7C0593BD0B,
    AK_SOUND_ENGINE_SET_LISTENER_SCALING_FACTOR_F6D9B99A3930,
    AK_SOUND_ENGINE_SET_LISTENER_SPATIALIZATION_F3E38FB62DBE,
    AK_MONITOR_SET_LOCAL_OUTPUT_7088FF58E8E6,
    AK_SOUND_ENGINE_SET_MAX_NUM_VOICES_LIMIT_FDB0FB30B8B5,
    AK_SOUND_ENGINE_SET_MEDIA_9E1BDFFAF79A,
    AK_SOUND_ENGINE_SET_MIXER_65E7BC4C183E,
    AK_SOUND_ENGINE_SET_MIXER_C05DF64D808B,
    AK_SOUND_ENGINE_SET_MIXER_D2D84390AC7E,
    AK_MEMORY_MGR_SET_MONITORING_9C2F4379A783,
    AK_SOUND_ENGINE_SET_MULTIPLE_POSITIONS_C21E5A42F749,
    AK_SOUND_ENGINE_SET_OBJECT_OBSTRUCTION_AND_OCCLUSION_98B692FBFDA8,
    AK_SOUND_ENGINE_SET_PANNING_RULE_E9F648015123,
    AK_MOTION_ENGINE_SET_PLAYER_LISTENER_401393052625,
    AK_MOTION_ENGINE_SET_PLAYER_VOLUME_0E836955CCE2,
    AK_MEMORY_MGR_SET_POOL_NAME_9F4D84CE545D,
    AK_MEMORY_MGR_SET_POOL_NAME_EEF0933C4318,
    AK_SOUND_ENGINE_SET_POSITION_6AF8247AFD54,
    AK_SOUND_ENGINE_SET_POSITION_INTERNAL_F2FAC3C13E59,
    CAK_RTPCMGR_SET_RTPCINTERNAL_414ED05CD68C,
    CAK_RTPCMGR_SET_RTPCINTERNAL_DFFBB40FA663,
    AK_SOUND_ENGINE_SET_RTPCVALUE_224B8C27AD2B,
    AK_SOUND_ENGINE_SET_RTPCVALUE_B50C10EA0A1A,
    AK_SOUND_ENGINE_SET_RTPCVALUE_67FEF9D2AC6E,
    AK_SOUND_ENGINE_SET_RTPCVALUE_BY_PLAYING_ID_BE7FFD7323AC,
    AK_SOUND_ENGINE_SET_RTPCVALUE_BY_PLAYING_ID_731F7B55D939,
    AK_SOUND_ENGINE_SET_RTPCVALUE_BY_PLAYING_ID_AC404947BBF4,
    AK_SOUND_ENGINE_SET_RANDOM_SEED_21A287AADD5E,
    AK_SOUND_ENGINE_SET_SECONDARY_OUTPUT_VOLUME_A120B97E25C9,
    AK_SOUND_ENGINE_SET_SPEAKER_ANGLES_5392B7255EF6,
    AK_SOUND_ENGINE_SET_STATE_AFACB292FBF1,
    AK_SOUND_ENGINE_SET_STATE_128B82BA0EB3,
    AK_SOUND_ENGINE_SET_STATE_B3B1172B53D3,
    AK_SOUND_ENGINE_SET_SWITCH_02C7DECC43D8,
    AK_SOUND_ENGINE_SET_SWITCH_F7F4F12BF736,
    AK_SOUND_ENGINE_SET_SWITCH_6D72557ADAD9,
    AK_SOUND_ENGINE_SET_VOLUME_THRESHOLD_0BCAD0D7D4D1,
    AK_SOUND_ENGINE_START_OUTPUT_CAPTURE_FC6370DC5B8D,
    AK_MEMORY_MGR_START_PROFILE_THREAD_USAGE_AF272D6CC4FA,
    AK_SOUND_ENGINE_START_PROFILER_CAPTURE_6DAB11C93B4C,
    AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_STOP_3CEF204C8D2E,
    AK_SOUND_ENGINE_STOP_ALL_C67520D79749,
    AK_SOUND_ENGINE_STOP_OUTPUT_CAPTURE_8865D78163FC,
    AK_SOUND_ENGINE_STOP_PLAYING_ID_187315448957,
    AK_MEMORY_MGR_STOP_PROFILE_THREAD_USAGE_C6862BD685B1,
    AK_SOUND_ENGINE_STOP_PROFILER_CAPTURE_EBF843AE6A95,
    AK_SOUND_ENGINE_STOP_SOURCE_PLUGIN_B44BDF831850,
    AK_SOUND_ENGINE_SUSPEND_44AF7646D030,
    AK_MEMORY_MGR_TERM_139C019C551C,
    AK_MUSIC_ENGINE_TERM_DDAC00E33B2D,
    AK_SOUND_ENGINE_TERM_2DB29D9832F3,
    AK_SOUND_ENGINE_UNLOAD_BANK_337F97155ED4,
    AK_SOUND_ENGINE_UNLOAD_BANK_9655D2964E83,
    AK_SOUND_ENGINE_UNLOAD_BANK_67235594F14F,
    AK_SOUND_ENGINE_UNLOAD_BANK_54C19FD1F2E0,
    AK_SOUND_ENGINE_UNLOAD_BANK_F45EED2F24D3,
    AK_SOUND_ENGINE_UNLOAD_BANK_59B0956756B0,
    AK_SOUND_ENGINE_DYNAMIC_SEQUENCE_UNLOCK_PLAYLIST_A2AE8B6FF790,
    AK_SOUND_ENGINE_UNREGISTER_ALL_GAME_OBJ_A57234B33D8B,
    AK_SOUND_ENGINE_UNREGISTER_GAME_OBJ_0468761A228E,
    AK_SOUND_ENGINE_UNREGISTER_GLOBAL_CALLBACK_9B4E4EBF2505,
    AK_SOUND_ENGINE_UNSET_MEDIA_74AE9F3BE0AF,
    AK_SOUND_ENGINE_WAKEUP_FROM_SUSPEND_0EE38A533A98,
    G_DEFAULT_POOL_ID_FC6F8C366D59,
    G_LENGINE_DEFAULT_POOL_ID_C2010F2CDC74,
    AK_SOUND_ENGINE_G_PLAYING_ID_2B5E59609F6B,
    G_P_ASSERT_HOOK_B24C36F35C65,
    G_P_RTPCMGR_B8D81F181C0D,
    G_P_REGISTRY_MGR_A5AE18348471,
    G_P_SWITCH_MGR_CA741FFE2530,
    AK_IAK_STREAM_MGR_M_P_STREAM_MGR_AB66837AE9D3,
    NV_OPTIMUS_ENABLEMENT_B6ED52D21305,
];

pub const ORDINAL_ONLY_EXPORT_COUNT: usize = 0;
