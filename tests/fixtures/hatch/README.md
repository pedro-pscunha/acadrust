# Hatch fixtures

`clockwise-arc-edge.dwg`: AutoCAD LT 2025 (core console) drew a 40 x 40 square whose right side has a
round bite (LINE, ARC center (20,0) radius 10, LINE, SPLINE top, LINE), then `-HATCH` SOLID at (0,0) and
`SAVEAS` 2018. The spline makes AutoCAD store an edge boundary. The bite arc is a clockwise edge: AutoCAD's
`DXFOUT` of the same drawing writes it as `50` = 90, `51` = 270, `73` = 0, and it runs from (20,-10) to
(20,10) through (10,0). So a clockwise edge stores 360 minus each real angle, in DWG as in DXF.
