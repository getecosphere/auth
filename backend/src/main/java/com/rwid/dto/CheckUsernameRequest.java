package com.rwid.dto;

import jakarta.validation.constraints.NotEmpty;

import java.util.List;

public record CheckUsernameRequest(
    @NotEmpty(message = "Username list cannot be empty")
    List<String> usernames
) {
}
