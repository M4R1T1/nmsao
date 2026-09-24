![alt text](assets/NMSAO_540p.png)

# **No Man's Sky Audio Organizer**
NMSAO is a portable application for browsing and exporting a human-readable reorganized list of all game audio files, creating projects to map sound file replacements, and importing external audio projects for conflict resolution purposes.

## **Important Notes**
- NMSAO is NOT an audio editor or creator. You must source your own audio files and edit them beforehand in software such as [Audacity](https://www.audacityteam.org/)
- Only WAV/WEM are supported as source files
- No Man's Sky exosuit VO is not supported. If you want to make your own fork of NMSAO to include them, you are welcome to do so
- NMSAO cannot guarantee that your edited sounds will sound correct in game. It does not modify or bypass any existing LFO or mixer settings that are already in place

## **Installation & Setup**

### **Requirements**
~6gb available disk space (if you already have unpacked game files, ~3gb)

### **Initial Setup** *(if you already have game files extracted, skip this step)*
- Download [HGPAKtool](https://github.com/monkeyman192/HGPAKtool) latest release and extract the zip to your preferred location
- Drag and dropNMSARC.audio.pak & NMSARC.audioBNK.pak from your game's PCBANKS folder onto hgpaktool.exe and wait for it to finish
- Move the created EXTRACTED folder up a level so it's in the GAMEDATA folder (optional, but neater)
- Delete the music folder inside EXTRACTED as well as pc_streaming.pck inside EXTRACTED/audio/windows (both unnneeded)

### **NMSAO Setup**
- Download nmsao.exe and place into a dedicated folder (not on the desktop)
- Launch nmsao.exe
- Open Project > Preferences, and paste in your No Man's Sky EXTRACTED folder path
- Click Process Assets > Process and wait for it to finish.
    - *When a new game update comes out, you will need to repeat the Initial Setup step with HGPAKtool in order for NMSAO to recognize any new audio files that have been added. You do not need to run the Clean Organized Audio Folders function after extracting updated files, but if you want another degree of certainty you can.*
    - *NMSAO does not read the game's pak files directly*

## **Usage**
### Taskbar
#### Project
- Preferences:
    - Default Source Path: The default path the File Explorer tab will use when starting the application. Click the += button to add the current path
    - EXTRACTED Folder Path: The path that the program will read for processing game assets, suffixed with audio/windows
    - Preserve organized folder structure when exporting game files to OGG: Checkbox for whether the folder structure you see in the Organized Audio tab will be created when exporting to OGG
- New:
    - Creates a new project
- Rename:
    - Renames the currently selected project
- Delete:
    - Deletes the currently selected project with confirmation check
    - If deleting an imported project, the source files will not be deleted in the imported folder
- Combine:
    - Combines two or more projects into a single project
    - First in order takes priority if linked sounds conflict
- Import:
    - Imports a built audio mod and its source files
    - If mapping.json exists the program will use that, otherwise imported source files will be generically named based of the top level folder of the imported mod
    - If the imported mod does not contain any audio files, an empty project will still be created
- Build Mod:
    - Build the currently selected project into a finalized mod, including mapping.json for reimport into NMSAO

#### Help
- Usage Guide: Opens the Github page link
- NMS Modding Discord: Opens a link to the NMS Modding Discord

#### Process Assets
- Process: Copies and reorganizes game files based on the EXTRACTED path in Preferences
- Clean Organized Audio Folders: Deletes the entire organized_audio folder and all its contents

### **Panels**
#### Source Files
- Built in file explorer used for linking your chosen source files (must be WAV or WEM)
- Green = wav/wem; source file can be linked and played with audio player
- Red = any other file; source file cannot be linked


#### Project
- Determines the actively loaded project
- X button for deleting individual links
- ⚠ button for deleting all links by source file (with countdown confirmation check)
- When a sound file is linked, you will see <> next to its Organized Audio listing to indicate it as such


#### Organized Audio
- Browsable hierarchical file tree
- Select sound(s) to queue, link, or silence
- Sounds can be selected in batches by:
    - Shift clicking
    - Ctrl clicking
    - Right clicking a folder which will allow you to queue or select all of its contents
- Right click an audio file to copy its original Id to clipboard
- When a sound file is linked, you will see <> next to its Organized Audio listing to indicate it as such


#### File Queue
- Another option for linking/silencing files all at once
- Used to export audio to ogg (cannot be done from Organized Audio tab)


#### Audio Player
- When a valid sound file (WAV/WEM) is selected in either Source Files or Organized Audio, this can be clicked to play them
- Playback controls:
    - Space: pause
    - Left Arrow: go back 10s
    - Right Arrow: go forward 10s


## **Credits & Acknowledgements**
- [wwiseutil](https://github.com/hpxro7/wwiseutil): Manipulation of Wwise soundbanks
- [BNKReplacer](https://github.com/EtiTheSpirit/BNKReplacer): Manipulation of Wwise soundbanks
- [egui](https://crates.io/crates/egui): UI framework for Rust
- [Audiokinetic](https://www.audiokinetic.com/en/): Creators of Wwise
- [HGPAKtool](https://github.com/monkeyman192/HGPAKtool): Unpacking No Man's Sky game files