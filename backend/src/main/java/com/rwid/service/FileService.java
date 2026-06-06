package com.rwid.service;

import com.rwid.model.File;
import com.rwid.repository.FileRepository;
import lombok.extern.slf4j.Slf4j;
import org.springframework.beans.factory.annotation.Value;
import org.springframework.stereotype.Service;
import org.springframework.web.multipart.MultipartFile;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.LocalDateTime;
import java.util.Locale;
import java.util.Set;
import java.util.UUID;

@Slf4j
@Service
public class FileService {
    private static final Set<String> IMAGE_FILE_TYPES = Set.of(
            "avatar",
            "avatars",
            "cover-photo",
            "cover-photos",
            "community-icons",
            "course-covers",
            "homepage-image",
            "logo",
            "post-image",
            "slide-image"
    );

    private final FileRepository fileRepository;

    @Value("${storage.type}")
    private String storageType;

    @Value("${storage.local.path}")
    private String localStoragePath;

    public FileService(FileRepository fileRepository) {
        this.fileRepository = fileRepository;
    }

    public String uploadFile(MultipartFile file, String fileType, String userId) throws IOException {
        String originalFilename = file.getOriginalFilename();
        String filename;
        byte[] fileBytes;
        long fileSize;
        String contentType = file.getContentType();

        if (shouldProcessAsImage(fileType, contentType)) {
            if (contentType == null || !contentType.startsWith("image/")) {
                throw new IllegalArgumentException("Invalid file type. Expected image upload for type: " + fileType);
            }

            filename = UUID.randomUUID().toString() + ".webp";
            log.info("Compressing image to WebP format for: {}", originalFilename);
            try {
                com.sksamuel.scrimage.ImmutableImage image = com.sksamuel.scrimage.ImmutableImage.loader().fromBytes(file.getBytes());
                fileBytes = image.bytes(com.sksamuel.scrimage.webp.WebpWriter.DEFAULT.withQ(80));
                fileSize = fileBytes.length;
                log.info("Image compressed to WebP: original size {} bytes -> compressed size {} bytes", file.getSize(), fileSize);
            } catch (Exception e) {
                log.error("Failed to convert image to WebP for {}", originalFilename, e);
                throw new IOException("Failed to convert image to WebP", e);
            }
        } else {
            filename = UUID.randomUUID() + "_" + originalFilename;
            fileBytes = file.getBytes();
            fileSize = file.getSize();
        }

        String storagePath = fileType + "/" + userId + "/" + filename;

        if ("local".equals(storageType)) {
            return uploadToLocalStorage(fileBytes, fileSize, storagePath, filename, fileType, userId);
        } else if ("s3".equals(storageType)) {
            return uploadToS3(fileBytes, fileSize, storagePath, filename, fileType, userId);
        } else {
            throw new IllegalArgumentException("Unknown storage type: " + storageType);
        }
    }

    private String uploadToLocalStorage(byte[] fileBytes, long fileSize, String storagePath, String filename, String fileType, String userId) throws IOException {
        Path uploadDir = Paths.get(localStoragePath, fileType, userId);
        Files.createDirectories(uploadDir);

        Path filePath = uploadDir.resolve(filename);
        Files.write(filePath, fileBytes);

        File fileRecord = File.builder()
                .filename(filename)
                .fileType(fileType)
                .fileSize(fileSize)
                .storageType("local")
                .storagePath(storagePath)
                .uploadedBy(userId)
                .createdAt(LocalDateTime.now())
                .build();

        File savedFile = fileRepository.save(fileRecord);
        log.info("File uploaded to local storage: {}", filename);

        return "/files/view/" + savedFile.getId();
    }

    private String uploadToS3(byte[] fileBytes, long fileSize, String storagePath, String filename, String fileType, String userId) throws IOException {
        // TODO: Implement S3 upload
        throw new UnsupportedOperationException("S3 upload not yet implemented");
    }

    public byte[] downloadFile(String fileId) throws IOException {
        File file = fileRepository.findById(fileId)
                .orElseThrow(() -> new IllegalArgumentException("File not found: " + fileId));

        if ("local".equals(storageType)) {
            Path filePath = Paths.get(localStoragePath, file.getStoragePath());
            return Files.readAllBytes(filePath);
        } else if ("s3".equals(storageType)) {
            // TODO: Implement S3 download
            throw new UnsupportedOperationException("S3 download not yet implemented");
        } else {
            throw new IllegalArgumentException("Unknown storage type: " + storageType);
        }
    }

    public void deleteFile(String fileId) throws IOException {
        File file = fileRepository.findById(fileId)
                .orElseThrow(() -> new IllegalArgumentException("File not found: " + fileId));

        if ("local".equals(storageType)) {
            Path filePath = Paths.get(localStoragePath, file.getStoragePath());
            Files.deleteIfExists(filePath);
        } else if ("s3".equals(storageType)) {
            // TODO: Implement S3 delete
            throw new UnsupportedOperationException("S3 delete not yet implemented");
        }

        fileRepository.deleteById(fileId);
        log.info("File deleted: {}", fileId);
    }

    public String getContentType(String fileId) {
        File file = fileRepository.findById(fileId)
                .orElseThrow(() -> new IllegalArgumentException("File not found: " + fileId));

        String filename = file.getFilename();
        if (filename.endsWith(".png")) {
            return "image/png";
        } else if (filename.endsWith(".jpg") || filename.endsWith(".jpeg")) {
            return "image/jpeg";
        } else if (filename.endsWith(".gif")) {
            return "image/gif";
        } else if (filename.endsWith(".webp")) {
            return "image/webp";
        } else if (filename.endsWith(".pdf")) {
            return "application/pdf";
        } else {
            return "application/octet-stream";
        }
    }

    private boolean shouldProcessAsImage(String fileType, String contentType) {
        if (contentType != null && contentType.startsWith("image/")) {
            return true;
        }
        return fileType != null && IMAGE_FILE_TYPES.contains(fileType.toLowerCase(Locale.ROOT));
    }
}
