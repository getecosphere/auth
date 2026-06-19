package com.rwid.dto;

import java.util.List;

public record CheckUsernameResponse(
    List<String> existing
) {
}
