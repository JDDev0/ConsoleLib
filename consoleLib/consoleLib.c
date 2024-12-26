#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <ctype.h>
#include "consoleLib.h"

#ifdef __unix__ //UNIX/Linux
    #include <curses.h>
    #include <unistd.h>
    #include <inttypes.h>

    static int TMP_KEY_F0 = CL_KEY_F1 - 1;
    static int columnTmp, rowTmp, columns, rows;
    static MEVENT *mev = NULL;
    static uint16_t colorIDMap[9][9]; //[FG][BG][Bit(0): hasColor, Bit(1 - 8): ID]
    static short lastColorPairID = 1;
    void clrscr(void) {
        columnTmp = 0;
        rowTmp = 0;
        refresh();
        erase();

        //Reset color
        lastColorPairID = 1;
        memset(colorIDMap, 0, sizeof(int16_t)*81);
    }

    void initConsole(void) {
        //Init draw
        initscr();
        noecho();
        curs_set(0);
        cbreak();
        timeout(0);
        set_escdelay(0);
        keypad(stdscr, TRUE);

        //Init color
        start_color();

        //Init mouse input
        mousemask(BUTTON1_PRESSED, NULL);
        mev = malloc(sizeof(MEVENT));

        //Get console size for draw
        getmaxyx(stdscr, rows, columns);

        //Force clear screen after init
        clear();
    }
    void reset(void) {
        clrscr();
        resetColor();
        setUnderline(0);

        if(mev) {
            free(mev);
            mev = NULL;
        }

        endwin();
    }

    void getConsoleSize(int *columnsRet, int *rowsRet) {
        *columnsRet = columns;
        *rowsRet = rows;
    }

    int hasInput(void) {
        const int ch = wgetch(stdscr);

        if(ch == ERR)
            return 0;

        ungetch(ch);

        if(ch == KEY_MOUSE)
            return 0;

        return 1;
    }
    int getKey(void) {
        const int ch = wgetch(stdscr);

        if(ch == KEY_MOUSE) {
            ungetch(ch);

            return ERR;
        }

        //Force redraw on resize
        if(ch == KEY_RESIZE) {
            clear();

            return ERR;
        }

        //Special keys
        switch(ch) {
            //Arrow keys
            case KEY_LEFT:
                return CL_KEY_LEFT;
            case KEY_UP:
                return CL_KEY_UP;
            case KEY_RIGHT:
                return CL_KEY_RIGHT;
            case KEY_DOWN:
                return CL_KEY_DOWN;
            //Other keys
            case 27:
                return CL_KEY_ESC;
            case KEY_BACKSPACE:
            case 127: //ASCII DEL
            case 8:   //ASCII BS
                return CL_KEY_DELETE;
            case '\n':
                return CL_KEY_ENTER;
            case '\t':
                return CL_KEY_TAB;

            default:
                break;
        }
        //F1 - F12
        for(int i = 1;i < 13;i++) {
            if(ch == KEY_F(i)) {
                return TMP_KEY_F0 + i;
            }
        }

        return tolower(ch);
    }

    void getMousePosClicked(int *column, int *row) {
        const int ch = wgetch(stdscr);
        if(ch == KEY_MOUSE) {
            if(getmouse(mev) == OK) {
                if(mev->bstate&BUTTON1_PRESSED) {
                    *column = mev->x;
                    *row = mev->y;

                    return;
                }
            }
        }

        *column = -1;
        *row = -1;
    }

    void drawText(const char *text) {
        int start = 0;
        int startColumn = columnTmp;
        const size_t len = strlen(text);
        for(int i = 0;i < len;i++) {
            columnTmp++;
            if(text[i] == '\n') {
                //Draw line
                //Remove '\n'
                mvaddnstr(rowTmp, startColumn, text + start, i - start);

                rowTmp++;
                startColumn = columnTmp = 0;
                start = i + 1;
            }
        }
        if(len > (size_t)start) {
            //Draw str without '\n'
            mvaddstr(rowTmp, startColumn, text + start);
        }
    }

    static int colorCodes[] = {
        COLOR_BLACK, COLOR_BLUE, COLOR_GREEN, COLOR_CYAN, COLOR_RED,
        COLOR_MAGENTA, COLOR_YELLOW, COLOR_WHITE
    };
    void setColor(const int fg, const int bg) {
        if(fg < -1 || fg > 15 || bg < -1 || bg > 15) {
            return;
        }

        resetColor();

        const int fgID = (fg == -1?-1:fg&7) + 1;
        const int bgID = (bg == -1?-1:bg&7) + 1;
        if(colorIDMap[fgID][bgID]&1) {
            attron(COLOR_PAIR(((colorIDMap[fgID][bgID] - 1 >> 1) & 127) + 1) | (fg&8||bg&8?(A_BOLD):0));
        }else {
            short fgCol;
            if(fg == -1) {
                fgCol = COLOR_BLACK;
            }else {
                fgCol = (short)colorCodes[fg&7];
            }
            short bgCol;
            if(bg == -1) {
                bgCol = COLOR_BLACK;
            }else {
                bgCol = (short)colorCodes[bg&7];
            }
            init_pair(lastColorPairID, fgCol, bgCol);
            attron(COLOR_PAIR(lastColorPairID)|((fg&8||bg&8)?(A_BOLD):0));

            //Set hasColor to 1 + (ID - 1)
            colorIDMap[fgID][bgID] |= ((uint8_t)(lastColorPairID - 1) << 1)|1;

            lastColorPairID++;
        }
    }
    void resetColor(void) {
        attroff(A_COLOR|A_BOLD);
    }

    void setUnderline(const int underline) {
        if(underline)
            attron(A_UNDERLINE);
        else
            attroff(A_UNDERLINE);
    }

    void setCursorPos(const int x, const int y) {
        columnTmp = x;
        rowTmp = y;
    }
#elif __WIN32__ || _MSC_VER //Windows
    #include <windows.h>
    #include <wincon.h>
    #include <conio.h>

    static HANDLE hConsole;
    static CONSOLE_SCREEN_BUFFER_INFO consoleInfo;
    static WORD savedAttributes;
    static CHAR_INFO *textBuf = NULL;
    static CHAR_INFO emptyChar;
    static WORD color;
    static int columns, rows;
    //Copy ASCII-Chars without color to CHAR_INFO[]
    static int columnTmp = 0, rowTmp = 0;
    //Mouse
    static HANDLE hInConsole;
    static INPUT_RECORD inBuf[32];
    static DWORD events;

    void clrscr(void) { //Draw and clear buffer after for next round
        textBuf[rows*columns].Char.AsciiChar = '\0';
        columnTmp = rowTmp = 0;
        const COORD pos = {0, 0};
        const COORD size = {(SHORT)columns, (SHORT)rows};
        SMALL_RECT windowBounds = {0, 0, (SHORT)(columns-1), (SHORT)(rows-1)};
        WriteConsoleOutputA(hConsole, textBuf, size, pos, &windowBounds);

        //Reset all chars (Clear screen)
        for(int i = 0;i < columns*rows;i++) {
            memcpy(textBuf + i, &emptyChar, sizeof(CHAR_INFO));
        }
    }

    void initConsole(void) {
        //Init Color
        hConsole = GetStdHandle(STD_OUTPUT_HANDLE);
        SetConsoleActiveScreenBuffer(hConsole);
        //Save old color
        GetConsoleScreenBufferInfo(hConsole, &consoleInfo);
        color = savedAttributes = consoleInfo.wAttributes;
        //Init empty char
        emptyChar.Char.AsciiChar = ' ';
        emptyChar.Attributes = savedAttributes;
        //Init text draw
        columns = consoleInfo.srWindow.Right - consoleInfo.srWindow.Left + 1;
        rows = consoleInfo.srWindow.Bottom - consoleInfo.srWindow.Top + 1;
        textBuf = malloc((unsigned)(columns*rows + 1)*sizeof(textBuf[0]));
        memset(textBuf, 0, (unsigned)(columns*rows + 1)*sizeof(textBuf[0]));
        //Init mouse input
        hInConsole = GetStdHandle(STD_INPUT_HANDLE);
        SetConsoleMode(hInConsole, ENABLE_EXTENDED_FLAGS|ENABLE_WINDOW_INPUT|
                       ENABLE_MOUSE_INPUT);
    }
    void reset(void) {
        clrscr();
        resetColor();
        SetConsoleTextAttribute(hConsole, savedAttributes);

        if(textBuf) {
            free(textBuf);
            textBuf = NULL;
        }
    }

    void getConsoleSize(int *columnsRet, int *rowsRet) {
        *columnsRet = columns;
        *rowsRet = rows;
    }

    int hasInput(void) {
        return _kbhit();
    }
    int getKey(void) {
        const int ch = _getch();

        //Arrow keys + F11 - F12
        if(ch == 224) {
            const int chTmp = _getch();
            switch(chTmp) {
                //Arrow keys
                case 72:
                    return CL_KEY_UP;
                case 75:
                    return CL_KEY_LEFT;
                case 77:
                    return CL_KEY_RIGHT;
                case 80:
                    return CL_KEY_DOWN;
                //F11 - F12
                case 133:
                    return CL_KEY_F11;
                case 134:
                    return CL_KEY_F12;

                default:
                    break;
            }
        }
        //Function key
        if(ch == 0) {
            const int chTmp = _getch();
            //F1 - F10
            for(int i = 0;i < 10;i++) {
                if(chTmp == 59 + i) {
                    return CL_KEY_F1 + i;
                }
            }
        }
        //Other keys
        switch(ch) {
            case VK_ESCAPE:
                return CL_KEY_ESC;
            case VK_BACK:
                return CL_KEY_DELETE;
            case VK_RETURN:
                return CL_KEY_ENTER;
            case VK_TAB:
                return CL_KEY_TAB;

            default:
                break;
        }

        return tolower(ch);
    }

    void getMousePosClicked(int *column, int *row) {
        //Get event count
        GetNumberOfConsoleInputEvents(hInConsole, &events);
        if(events != 0)
          ReadConsoleInput(hInConsole, inBuf, events, &events);

        for(int i = 0;i < (int)events;i++) {
            if(inBuf[i].EventType == MOUSE_EVENT) {
                switch(inBuf[i].Event.MouseEvent.dwEventFlags) {
                    case 0:
                        if(inBuf[i].Event.MouseEvent.dwButtonState &
                        FROM_LEFT_1ST_BUTTON_PRESSED) {
                            *column = inBuf[i].Event.MouseEvent.dwMousePosition.X;
                            *row = inBuf[i].Event.MouseEvent.dwMousePosition.Y;

                            return;
                        }

                        break;

                    default:
                        break;
                }
            }
        }

        *column = -1;
        *row = -1;
    }

    void drawText(const char *text) {
        //Copy to textBuf
        const signed len = (signed)strlen(text);
        for(int i = 0;i < len;i++) {
            if(text[i] == '\n') {
                rowTmp++;
                columnTmp = 0;

                continue;
            }

            if(rowTmp*columns + columnTmp > columns*rows + 1) {
                //Prevent out of bounds write

                break;
            }

            textBuf[rowTmp*columns + columnTmp].Char.AsciiChar = text[i];
            textBuf[rowTmp*columns + columnTmp].Attributes = color;

            columnTmp++;
        }
    }

    void setColor(const int fg, const int bg) {
        if(fg < -1 || fg > 15 || bg < -1 || bg > 15) {
            return;
        }

        //a: {"alpha": 0: dark, 1: light}
        const int aFG = (fg>>3)%2;
        const int rFG = (fg>>2)%2;
        const int gFG = (fg>>1)%2;
        const int bFG = fg%2;

        //a: {"alpha": 0: dark, 1: light}
        const int aBG = (bg>>3)%2;
        const int rBG = (bg>>2)%2;
        const int gBG = (bg>>1)%2;
        const int bBG = bg%2;

        //Reset color
        color = color & ~(FOREGROUND_INTENSITY|FOREGROUND_RED|FOREGROUND_GREEN|FOREGROUND_BLUE|
            BACKGROUND_INTENSITY|BACKGROUND_RED|BACKGROUND_GREEN|BACKGROUND_BLUE);

        //Set color
        if(fg != -1) {
            if(aFG) {
                color |= FOREGROUND_INTENSITY;
            }
            if(rFG) {
                color |= FOREGROUND_RED;
            }
            if(gFG) {
                color |= FOREGROUND_GREEN;
            }
            if(bFG) {
                color |= FOREGROUND_BLUE;
            }
        }else { //Foreground color from savedAttributes
            color |= savedAttributes & (FOREGROUND_INTENSITY|FOREGROUND_RED|FOREGROUND_GREEN|FOREGROUND_BLUE);
        }
        if(bg != -1) {
            if(aBG) {
                color |= BACKGROUND_INTENSITY;
            }
            if(rBG) {
                color |= BACKGROUND_RED;
            }
            if(gBG) {
                color |= BACKGROUND_GREEN;
            }
            if(bBG) {
                color |= BACKGROUND_BLUE;
            }
        }else { //Background color from savedAttributes
            color |= savedAttributes & (BACKGROUND_INTENSITY|BACKGROUND_RED|BACKGROUND_GREEN|BACKGROUND_BLUE);
        }
    }
    void resetColor(void) {
        color = savedAttributes;
    }

    void setUnderline(const int underline) {
        if(underline) {
            color |= COMMON_LVB_UNDERSCORE;
        }else {
            color = color & ~COMMON_LVB_UNDERSCORE;
        }
    }

    void setCursorPos(const int x, const int y) {
        columnTmp = x;
        rowTmp = y;
    }
#endif
