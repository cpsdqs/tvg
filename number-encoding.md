# tvg rational number encoding
interestingly, if you zoom in far enough in the camera view and drag points around, the coordinates seem to be moving on a grid, which is very convenient for decoding!

samples from dragging a point around and recording its value in the resulting tvg file:
```
bytes = multiplier => tvg2xml value

BD A0 00 00 = -5 => -72
BD 80 00 00 = -4 => -64
BD 40 00 00 = -3 => -48
BD 00 00 00 = -2 => -32
BC 80 00 00 = -1 => -16
00 00 00 00 =  0 => 0
3C 80 00 00 =  1 => 16
3D 00 00 00 =  2 => 32
3D 40 00 00 =  3 => 48
3D 80 00 00 =  4 => 64
3D A0 00 00 =  5 => 72
3D C0 00 00 =  6 => 96
3D E0 00 00 =  7 => 112
3E 00 00 00 =  8 => 128
3E 10 00 00 =  9 => 144
3E 20 00 00 = 10 => 160
3E 30 00 00 = 11 => 176
3E 40 00 00 = 12 => 192
3E 50 00 00 = 13 => 208
3E 60 00 00 = 14 => 224
3E 70 00 00 = 15 => 240
3E 80 00 00 = 16 => 256
3E 88 00 00 = 17 => 272
3E 90 00 00 = 18 => 288
3E 98 00 00 = 19 => 304
3E A0 00 00 = 20 => 320
3E A8 00 00 = 21 => 336
3E B0 00 00 = 22 => 352
3E B8 00 00 = 23 => 368
3E C0 00 00 = 24 => 384
3E C8 00 00 = 25 => 400
3E D0 00 00 = 26 => 416
3E D8 00 00 = 27 => 432
3E E0 00 00 = 28 => 448
3E E8 00 00 = 29 => 464
3E F0 00 00 = 30 => 480
3E F8 00 00 = 31 => 496
3F 00 00 00 = 32 => 512
3F 04 00 00 = 33 => 528
3F 08 00 00 = 34 => 544
3F 0C 00 00 = 35 => 560
3F 10 00 00 = 36 => 576
3F 14 00 00 = 37 => 592
3F 18 00 00 = 38
3F 1C 00 00 = 39
3F 20 00 00 = 40
3F 24 00 00 = 41
3F 28 00 00 = 42
3F 2C 00 00 = 43
3F 30 00 00 = 44
3F 34 00 00 = 45
3F 38 00 00 = 46
3F 3C 00 00 = 47
3F 40 00 00 = 48
3F 44 00 00 = 49
3F 48 00 00 = 50
3F 4C 00 00 = 51
3F 50 00 00 = 52
3F 54 00 00 = 53
3F 58 00 00 = 54
3F 5C 00 00 = 55
3F 60 00 00 = 56
3F 64 00 00 = 57
3F 68 00 00 = 58
3F 6C 00 00 = 59
3F 70 00 00 = 60
3F 74 00 00 = 61
3F 78 00 00 = 62
3F 7C 00 00 = 63 => 1008
3F 80 00 00 = 64 => 1r0
3F 82 00 00 =  0 => 1r16
3F 84 00 00 =  1 => 1r32
3F 86 00 00 =  2 => 1r64
3F 88 00 00 =  3 => 1r72
3F 8A 00 00 =  4 => 1r80
3F 8C 00 00 =  5 => 1r96
3F 8E 00 00 =  6
3F 90 00 00 =  7
3F 92 00 00 =  8
...
3F FE 00 00 = 63 => 1r1008
40 00 00 00 =  0 => 2r0
40 01 00 00 =  1 => 2r16
40 02 00 00 =  2
40 03 00 00 =  3
40 04 00 00 =  4
...
40 80 00 00 =  0 => 4r0
40 80 80 00 =  1 => 4r16
40 81 00 00 =  2 => 4r32
40 81 80 00 =  3 => 4r48
40 82 00 00 =  4 => 4r48
```

This is a very strange format unlike any float or fixed-point encoding I’ve been able to find…
if you stare at the data long enough you can make out the pattern:

- negative number `B` = `1011`
- positive number `3` = `0011`
- => 1 bit sign
- => 8 bits “exponent”
    - (`3C8`) `011 1100 1` spans [1/64, 2/64[ / 0 bits value
    - (`3D0`) `011 1101 0` spans [2/64, 4/64[ / 1 bit
    - (`3D8`) `011 1101 0` spans [4/64, 8/64[ / 2 bits
    - (`3E0`) `011 1110 0` spans [8/64, 16/64[ / 3 bits
    - (`3E8`) `011 1110 1` spans [16/64, 32/64[ / 4 bits
    - (`3F0`) `011 1111 0` spans [32/64, 1[ / 5 bits
    - (`3F8`) `011 1111 1` spans [1, 2[ / 6 bit
    - (`400`) `100 0000 0` spans [2, 4[ / 7 bit
    - speculation:
    - (`408`) `100 0000 1` spans [4, 8[ / 8 bit
    - (`410`) `100 0001 0` spans [8, 16[ / 9 bit
    - (`418`) `100 0001 1` spans [16, 32[ / 10 bit
    - ...

Hence:
```
1 bit sign, 8 bits exponent, 23 bits fractional value

|x| = 2 ^ (exponent - 0x7F) + [first (exponent - 0x79) bits of fractional value] / 64

special case: all bits 0 => x = 0
```

I’m not sure what’d happen when you use up all 23 bits of the fractional value, as that would still be a scenario encodable in the 8-bit exponent.
However, those values would have to be really big, so it probably doesn’t matter.

