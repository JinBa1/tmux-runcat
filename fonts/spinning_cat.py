"""Original, tiny rotating cat geometry for the OIIA-inspired expression."""

import math


def pixels(frame, count):
    # Orthographic yaw, rather than a cartwheel: a slim body, four paws,
    # pointed ears, a curled tail and eyes which disappear behind the head.
    angle = frame * math.tau / count
    cosine, sine = math.cos(angle), math.sin(angle)
    shapes = [
        ((0, 6, -2), (4.0, 4.8, 8.5), True),
        ((0, 10, 6), (3.7, 3.5, 3.8), True),
    ]
    for side in (-1, 1):
        for depth in (-6, 6):
            shapes.append(((side * 2.9, 2, depth), (1.25, 2, 1.5), True))
        for step in range(7):
            shapes.append(((side * 2.6, 12 + step * 0.5, 6),
                           (1.4 - step * 0.18, 0.6, 1.1 - step * 0.12), True))
        shapes.append(((side * 1.5, 10.8, 9.45), (0.65, 0.75, 0.5), False))
    for step in range(9):
        shapes.append(((step * 0.6, 3 + step * 0.3, -10.5 - step * 0.25),
                       (0.7, 0.7, 0.7), True))

    result = []
    for row in range(50):
        height = 49 - row - 18
        line = []
        for column in range(50):
            horizontal = column + 0.5 - 25
            closest, visible = -math.inf, False
            for center, radius, ink in shapes:
                origin = (horizontal * cosine - center[0], height - center[1],
                          horizontal * sine - center[2])
                direction = (-sine, 0, cosine)
                a = sum((direction[i] / radius[i]) ** 2 for i in range(3))
                b = 2 * sum(origin[i] * direction[i] / radius[i] ** 2 for i in range(3))
                c = sum((origin[i] / radius[i]) ** 2 for i in range(3)) - 1
                discriminant = b * b - 4 * a * c
                if discriminant >= 0:
                    depth = (-b + math.sqrt(discriminant)) / (2 * a)
                    if depth > closest:
                        closest, visible = depth, ink
            line.append((255, 160, 60, 255) if visible else (0, 0, 0, 0))
        result.append(line)
    return result
