// Generate native pen packets independently of Rust, using the Windows SDK.
// Run in an x64 or x86 Native Tools command prompt from the repository root:
//   cl /nologo /W4 /WX tests\fixtures\windows_pen_fixture.c /Fe:target\pen-fixture.exe /Fo:target\pen-fixture.obj
//   target\pen-fixture.exe tests\fixtures\windows-pen-x64.bin
// Use windows-pen-x86.bin when compiling with the x86 toolchain.
#define WIN32_LEAN_AND_MEAN
#define _WIN32_WINNT 0x0602
#include <windows.h>
#include <stddef.h>
#include <stdio.h>

#define PRINT_OFFSET(type, field) printf(#type "." #field "=%zu\n", offsetof(type, field))

int main(int argc, char **argv) {
    POINTER_PEN_INFO history[2];
    FILE *output = NULL;
    size_t written;
    int closed;
    unsigned int i;

    if (argc != 2) return 2;
    ZeroMemory(history, sizeof(history));
    // Like GetPointerPenInfoHistory, the newest record comes first.
    for (i = 0; i < 2; ++i) {
        POINTER_PEN_INFO *pen = &history[i];
        POINTER_INFO *pointer = &pen->pointerInfo;
        pointer->pointerType = PT_PEN;
        pointer->pointerId = 7;
        pointer->frameId = 101 - i;
        pointer->pointerFlags = POINTER_FLAG_INRANGE | POINTER_FLAG_INCONTACT
            | POINTER_FLAG_FIRSTBUTTON | POINTER_FLAG_UPDATE;
        pointer->sourceDevice = (HANDLE)(UINT_PTR)0x10203;
        pointer->hwndTarget = (HWND)(UINT_PTR)0x40506;
        pointer->ptPixelLocation.x = i == 0 ? 250 : 210;
        pointer->ptPixelLocation.y = i == 0 ? 130 : 120;
        pointer->ptHimetricLocation.x = 12345;
        pointer->ptHimetricLocation.y = 67890;
        pointer->ptPixelLocationRaw.x = pointer->ptPixelLocation.x - 1;
        pointer->ptPixelLocationRaw.y = pointer->ptPixelLocation.y - 1;
        pointer->ptHimetricLocationRaw.x = 12300;
        pointer->ptHimetricLocationRaw.y = 67800;
        pointer->dwTime = 1234 - i;
        pointer->historyCount = 2;
        pointer->InputData = -9;
        pointer->dwKeyStates = 0;
        pointer->PerformanceCount = 1001 - i;
        pointer->ButtonChangeType = POINTER_CHANGE_NONE;
        pen->penFlags = PEN_FLAG_NONE;
        pen->penMask = PEN_MASK_PRESSURE | PEN_MASK_ROTATION | PEN_MASK_TILT_X | PEN_MASK_TILT_Y;
        pen->pressure = i == 0 ? 1024 : 256;
        pen->rotation = 90;
        pen->tiltX = -23;
        pen->tiltY = 11;
    }

    if (fopen_s(&output, argv[1], "wb") != 0) return 3;
    written = fwrite(history, sizeof(history), 1, output);
    closed = fclose(output);
    if (written != 1 || closed != 0) return 4;

    printf("POINTER_INFO.size=%zu align=%zu\n", sizeof(POINTER_INFO), (size_t)__alignof(POINTER_INFO));
    printf("POINTER_PEN_INFO.size=%zu align=%zu\n", sizeof(POINTER_PEN_INFO), (size_t)__alignof(POINTER_PEN_INFO));
    PRINT_OFFSET(POINTER_INFO, sourceDevice);
    PRINT_OFFSET(POINTER_INFO, hwndTarget);
    PRINT_OFFSET(POINTER_INFO, ptPixelLocation);
    PRINT_OFFSET(POINTER_INFO, ptHimetricLocation);
    PRINT_OFFSET(POINTER_INFO, ptPixelLocationRaw);
    PRINT_OFFSET(POINTER_INFO, ptHimetricLocationRaw);
    PRINT_OFFSET(POINTER_INFO, dwTime);
    PRINT_OFFSET(POINTER_INFO, historyCount);
    PRINT_OFFSET(POINTER_INFO, InputData);
    PRINT_OFFSET(POINTER_INFO, dwKeyStates);
    PRINT_OFFSET(POINTER_INFO, PerformanceCount);
    PRINT_OFFSET(POINTER_INFO, ButtonChangeType);
    PRINT_OFFSET(POINTER_PEN_INFO, penFlags);
    PRINT_OFFSET(POINTER_PEN_INFO, penMask);
    PRINT_OFFSET(POINTER_PEN_INFO, pressure);
    PRINT_OFFSET(POINTER_PEN_INFO, rotation);
    PRINT_OFFSET(POINTER_PEN_INFO, tiltX);
    PRINT_OFFSET(POINTER_PEN_INFO, tiltY);
    return 0;
}
