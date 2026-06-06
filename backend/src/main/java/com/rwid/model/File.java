package com.rwid.model;

import lombok.AllArgsConstructor;
import lombok.Builder;
import lombok.Data;
import lombok.NoArgsConstructor;
import org.springframework.data.annotation.Id;
import org.springframework.data.mongodb.core.mapping.Document;

import java.time.LocalDateTime;

@Data
@NoArgsConstructor
@AllArgsConstructor
@Builder
@Document(collection = "files")
public class File {
    
    @Id
    private String id;
    
    private String filename;
    
    private String fileType;
    
    private long fileSize;
    
    private String storageType; // local or s3
    
    private String storagePath;
    
    private String uploadedBy;
    
    private LocalDateTime createdAt;
}
