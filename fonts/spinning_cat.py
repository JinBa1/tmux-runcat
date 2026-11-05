"""Original, tiny rotating cat geometry for the OIIA-inspired expression."""

import math
import cat_coats


def pixels(frame, count, fur=None):
    # Orthographic yaw, rather than a cartwheel: a slim body, four paws,
    # pointed ears, a curled tail and eyes which disappear behind the head.
    angle = frame * math.tau / count
    cosine, sine = math.cos(angle), math.sin(angle)
    coat = cat_coats.get(fur)
    shapes = [
        ((0, 6, -2), (4.0, 4.8, 8.5), "body"),
        ((0, 10, 6), (3.7, 3.5, 3.8), "head"),
    ]
    for side in (-1, 1):
        for depth in (-6, 6):
            shapes.append(((side * 2.9, 2, depth), (1.25, 2, 1.5), "leg"))
        for step in range(7):
            shapes.append(((side * 2.6, 12 + step * 0.5, 6),
                           (1.4 - step * 0.18, 0.6, 1.1 - step * 0.12), "ear"))
        shapes.append(((side * 1.5, 10.8, 9.45), (0.65, 0.75, 0.5), "eye"))
    for step in range(9):
        shapes.append(((step * 0.6, 3 + step * 0.3, -10.5 - step * 0.25),
                       (0.7, 0.7, 0.7), "tail"))

    result = []
    for row in range(50):
        height = 49 - row - 18
        line = []
        for column in range(50):
            horizontal = column + 0.5 - 25
            closest, color = -math.inf, (0, 0, 0, 0)
            for center, radius, part in shapes:
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
                        closest = depth
                        if coat:
                            local = tuple(origin[i] + depth * direction[i] for i in range(3))
                            point = tuple(local[i] + center[i] for i in range(3))
                            normal = tuple(local[i] / radius[i] for i in range(3))
                            color = coat.surface(part, point, normal)
                        else:
                            color = (0, 0, 0, 0) if part == "eye" else (255, 160, 60, 255)
            line.append(color)
        result.append(line)
    return result
