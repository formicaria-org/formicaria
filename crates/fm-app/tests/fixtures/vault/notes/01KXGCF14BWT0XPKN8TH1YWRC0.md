---
schema: 1
id: 01KXGCF14BWT0XPKN8TH1YWRC0
type: note
title: GAE and inner-loop adaptation
status: doing
created: 2026-07-14T13:00:00Z
updated: 2026-07-14T13:00:00Z
tags:
- meta-rl
---
# GAE and inner-loop adaptation

The GAE lambda interacts badly with inner-loop adaptation. Inline $\lambda = 0.95$ and display:

$$A_t = \sum_{l=0}^{\infty} (\gamma\lambda)^l \delta_{t+l}$$

```mermaid
graph LR; sample --> inner_loop --> meta_update --> sample
```

![trust-region figure](asset:sha256-deadbeef)

| step | note               |
| ---- | ------------------ |
| 1    | pin the tokenizer  |
| 2    | measure worst case |

- pin `unicode61 remove_diacritics 2`
- measure the worst case
