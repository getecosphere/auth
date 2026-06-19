package com.rwid.repository;

import com.rwid.model.User;
import org.springframework.data.mongodb.repository.MongoRepository;
import org.springframework.data.mongodb.repository.Query;
import org.springframework.stereotype.Repository;

import java.util.List;
import java.util.Optional;

@Repository
public interface UserRepository extends MongoRepository<User, String> {
    
    Optional<User> findByUsername(String username);
    
    Optional<User> findByEmail(String email);
    
    @Query("{ 'platformId': ?0, 'deletedAt': null }")
    List<User> findByPlatformId(String platformId);
    
    @Query("{ 'platformId': ?0, 'name': { $regex: ?1, $options: 'i' }, 'deletedAt': null }")
    List<User> searchByNameInPlatform(String platformId, String query);
    
    @Query("{ 'username': { $in: ?0 }, 'deletedAt': null }")
    List<User> findByUsernameIn(List<String> usernames);
}
