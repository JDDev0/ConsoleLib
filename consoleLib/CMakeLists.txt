cmake_minimum_required(VERSION 3.10)
project(ConsoleLib C)

set(CMAKE_C_STANDARD 11)

include_directories(.)

add_library(ConsoleLib
        consoleLib.c
        consoleLib.h)

install(TARGETS ConsoleLib DESTINATION .)

if(UNIX)
    find_package(Curses REQUIRED)
    include_directories(${CURSES_INCLUDE_DIR})

    target_link_libraries(ConsoleLib ${CURSES_LIBRARIES})
endif()
