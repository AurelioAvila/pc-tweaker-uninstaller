PC Tweaker Uninstaller 0.11.1 stops taking Pro away from customers who are paid up, and makes the primary button readable.

A cached licence was trusted for three days from the moment it was issued, and that window applied whether or not the signature said the customer had paid through next March. Four days without a reachable backend therefore revoked Pro from people who were paid up, with no way for them to prove it: leftover cleaning and batch removal are the two things Pro buys, and both stopped.

- A licence whose payload names a paid period still ahead of us now holds for thirty days without a refresh, and never past the date the backend actually signed. The three-day window remains for licences that state no expiry.
- The primary button carried white text on the accent gradient, which fails WCAG AA on all eight themes - 3.46:1 on Violet at best and 1.48:1 on Slate Mono, against a 4.5:1 requirement. It is dark ink now, which clears the requirement everywhere, and a check in CI fails the build if a future palette breaks it again.

Windows application and installer signatures are verified before distribution. Code signing identifies the publisher; Windows may still show reputation warnings.
