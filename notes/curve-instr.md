# curve instructions
`TGBP` point data is preceded by several bytes of what appears to be curve instructions.

samples with tvg2xml:
```
2 points - line
0000 0011 / 03
reverse: 1 1

3 points - ??? guess: polyline
0000 0111 / 07
reverse: 1 1 1

4 points - all cubic
0000 1001 / 09
reverse: 1 001

7 points - all cubic
0100 1001 / 49
reverse: 1 001 001

10 points - all cubic
0100 1001 / 49
0000 0010 / 02
reverse: 1 001 001 001

11 points - cubic, cubic, cubic, line
0100 1001 / 49
0000 0110 / 06
reverse: 1 001 001 001 1

13 points - all cubic
0100 1001 / 49
0001 0010 / 12
reverse: 1 001 001 001 001

16 points - all cubic
0100 1001 / 49
1001 0010 / 92
reverse: 1 001 001 001 001 001

19 points - all cubic
0100 1001 / 49
1001 0010 / 92
0000 0100 / 04
reverse: 1 001 001 001 001 001 001

22 points - all cubic
0100 1001 / 49
1001 0010 / 92
0010 0100 / 24
reverse: 1 001 001 001 001 001 001 001

25 points - all cubic
0100 1001 / 49
1001 0010 / 92
0010 0100 / 24
0000 0001 / 01
reverse: 1 001 001 001 001 001 001 001 001

31 points - all cubic
0100 1001 / 49
1001 0010 / 92
0010 0100 / 24
0100 1001 / 49
reverse: 1 001 001 001 001 001 001 001 001 001 001

32 points - line, cubic x 10
1001 0011 / 93
0010 0100 / 24
0100 1001 / 49
1001 0010 / 92
reverse: 1 1 001 001 001 001 001 001 001 001 001 001
```

It seems that if you read this LSB to MSB, it encodes each path segment's type!
