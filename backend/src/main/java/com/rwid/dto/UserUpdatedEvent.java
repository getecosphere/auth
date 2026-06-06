package com.rwid.dto;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;

import java.time.LocalDateTime;

@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
public class UserUpdatedEvent {
    private String eventId;
    private String eventType; // USER_CREATED, USER_UPDATED, USER_DELETED
    private LocalDateTime timestamp;
    private UserDTO user;
}
