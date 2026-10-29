"""Une page calme et claire, pour mesurer le coût de l'îlot au-dessus d'une
fenêtre de travail (pas 8, `DECISION-APRES-EPREUVE.md` A3 : « verre choisi,
îlot déployé sur la page PDF claire à 1,25, fenêtre calme »).

    python outils/page-calme.py <drapeau> [--duree=900] [--gris=255]

Une fenêtre d'une seule couleur (blanc par défaut) sur toute la zone de
travail — pas le moniteur entier, que le système lirait comme une application
en plein écran et dont la règle retirerait l'îlot —, sans curseur
qui clignote, sans animation, sans barre de titre : rien n'y change d'une image
à l'autre, et la capture du verre n'a rien à copier. Elle pompe ses messages ;
quand le fichier `<drapeau>` apparaît, elle se passe devant (son fil s'attache
à celui de la fenêtre au premier plan, `AttachThreadInput`, ce qui lui donne le
droit de poser le premier plan malgré le verrou de Windows sans envoyer
d'entrée ; une entrée nulle ne sert qu'en secours), puis efface le drapeau. Elle se ferme au
bout de `--duree` secondes, ou si le fichier `<drapeau>.fin` apparaît.
"""

import ctypes
import os
import sys
import time
from ctypes import wintypes

user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32
gdi32 = ctypes.windll.gdi32
user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))

WNDPROC = ctypes.WINFUNCTYPE(ctypes.c_ssize_t, wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM)
user32.DefWindowProcW.restype = ctypes.c_ssize_t
user32.DefWindowProcW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
user32.CreateWindowExW.restype = wintypes.HWND
user32.CreateWindowExW.argtypes = [wintypes.DWORD, wintypes.LPCWSTR, wintypes.LPCWSTR, wintypes.DWORD, ctypes.c_int,
                                   ctypes.c_int, ctypes.c_int, ctypes.c_int, wintypes.HWND, wintypes.HMENU,
                                   wintypes.HINSTANCE, wintypes.LPVOID]
gdi32.CreateSolidBrush.restype = wintypes.HANDLE


class WNDCLASSW(ctypes.Structure):
    _fields_ = [("style", wintypes.UINT), ("lpfnWndProc", WNDPROC), ("cbClsExtra", ctypes.c_int),
                ("cbWndExtra", ctypes.c_int), ("hInstance", wintypes.HINSTANCE), ("hIcon", wintypes.HANDLE),
                ("hCursor", wintypes.HANDLE), ("hbrBackground", wintypes.HANDLE), ("lpszMenuName", wintypes.LPCWSTR),
                ("lpszClassName", wintypes.LPCWSTR)]


class MOUSEINPUT(ctypes.Structure):
    _fields_ = [("dx", wintypes.LONG), ("dy", wintypes.LONG), ("mouseData", wintypes.DWORD),
                ("dwFlags", wintypes.DWORD), ("time", wintypes.DWORD), ("dwExtraInfo", ctypes.c_size_t)]


class INPUT(ctypes.Structure):
    class _U(ctypes.Union):
        _fields_ = [("mi", MOUSEINPUT), ("pad", ctypes.c_byte * 32)]
    _anonymous_ = ("u",)
    _fields_ = [("type", wintypes.DWORD), ("u", _U)]


def entree_nulle():
    i = INPUT()
    i.type = 0
    i.mi = MOUSEINPUT(0, 0, 0, 0x0001, 0, 0)
    user32.SendInput(1, ctypes.byref(i), ctypes.sizeof(INPUT))


def main():
    drapeau = os.path.abspath(sys.argv[1])
    opts = {a.split("=", 1)[0]: (a.split("=", 1)[1] if "=" in a else True) for a in sys.argv[2:]}
    duree = float(opts.get("--duree", 900))
    gris = int(opts.get("--gris", 255))
    proc = WNDPROC(lambda h, m, w, l: user32.DefWindowProcW(h, m, w, l))
    brosse = gdi32.CreateSolidBrush(gris | (gris << 8) | (gris << 16))
    wc = WNDCLASSW(0, proc, 0, 0, kernel32.GetModuleHandleW(None), None,
                   user32.LoadCursorW(None, ctypes.c_void_p(32512)), brosse, None, "PageCalmeVeilleuse")
    user32.RegisterClassW(ctypes.byref(wc))
    WS_POPUP, WS_VISIBLE, WS_MINIMIZEBOX, WS_EX_APPWINDOW = 0x80000000, 0x10000000, 0x20000, 0x40000
    # La zone de travail, pas le moniteur entier : une fenetre qui couvre tout
    # le moniteur est lue par le systeme comme une application en plein ecran
    # (SHQueryUserNotificationState = occupe), et la regle de l'ilot le
    # retirerait (mesure : pas8/essai-1, « Page calme -> retire »).
    zone = wintypes.RECT()
    user32.SystemParametersInfoW(0x0030, 0, ctypes.byref(zone), 0)  # SPI_GETWORKAREA
    l, h = zone.right - zone.left, zone.bottom - zone.top
    hwnd = user32.CreateWindowExW(WS_EX_APPWINDOW, "PageCalmeVeilleuse", "Page calme", WS_POPUP | WS_VISIBLE | WS_MINIMIZEBOX,
                                  zone.left, zone.top, l, h, None, None, kernel32.GetModuleHandleW(None), None)
    print(f"page calme : {l}x{h} a ({zone.left}, {zone.top}), la zone de travail", flush=True)
    fin = time.time() + duree
    msg = wintypes.MSG()
    dernier = 0.0
    while time.time() < fin and not os.path.exists(drapeau + ".fin"):
        while user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, 1):
            user32.TranslateMessage(ctypes.byref(msg))
            user32.DispatchMessageW(ctypes.byref(msg))
        if time.time() - dernier > 0.25:
            dernier = time.time()
            if os.path.exists(drapeau):
                # Le droit de poser le premier plan, sans entree : le fil de la
                # page s'attache a celui de la fenetre au premier plan
                # (AttachThreadInput), pose, se detache. Une entree nulle
                # compterait comme une entree de l'auteur pour qui mesure son
                # inactivite (GetLastInputInfo) ; elle ne sert qu'en secours.
                devant = user32.GetForegroundWindow()
                fil_devant = user32.GetWindowThreadProcessId(devant, None)
                fil = kernel32.GetCurrentThreadId()
                attache = bool(user32.AttachThreadInput(fil_devant, fil, True)) if devant and fil_devant != fil else False
                ok = user32.SetForegroundWindow(hwnd)
                if attache:
                    user32.AttachThreadInput(fil_devant, fil, False)
                if not ok:
                    entree_nulle()
                    ok = user32.SetForegroundWindow(hwnd)
                try:
                    os.remove(drapeau)
                except OSError:
                    pass
                print(f"page calme devant : {bool(ok)}", flush=True)
        time.sleep(0.01)
    user32.DestroyWindow(hwnd)


if __name__ == "__main__":
    main()
