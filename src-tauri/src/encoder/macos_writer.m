#import <AVFoundation/AVFoundation.h>
#import <VideoToolbox/VideoToolbox.h>

@interface EncoreWriterBox : NSObject
@property(nonatomic, strong) AVAssetWriter *writer;
@property(nonatomic, strong) AVAssetWriterInput *input;
@property(nonatomic, strong) AVAssetWriterInputPixelBufferAdaptor *adaptor;
@end

@implementation EncoreWriterBox
@end

static int copy_error(char *buffer, size_t capacity, NSString *message, int code) {
  if (buffer != NULL && capacity > 0) {
    const char *text = message.UTF8String ?: "unknown";
    snprintf(buffer, capacity, "%s", text);
  }
  return code;
}

void *encore_writer_create(const char *path, uint32_t width, uint32_t height,
                           char *error, size_t error_capacity) {
  @autoreleasepool {
    NSString *outputPath = [NSString stringWithUTF8String:path];
    NSError *writerError = nil;
    AVAssetWriter *writer =
        [AVAssetWriter assetWriterWithURL:[NSURL fileURLWithPath:outputPath]
                                fileType:AVFileTypeMPEG4
                                   error:&writerError];
    if (writer == nil) {
      copy_error(error, error_capacity, writerError.localizedDescription, 1);
      return NULL;
    }

    NSDictionary *encoder = @{
      (__bridge NSString *)kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder : @YES
    };
    NSDictionary *compression = @{
      AVVideoAverageBitRateKey : @3000000,
      AVVideoMaxKeyFrameIntervalKey : @30,
      AVVideoMaxKeyFrameIntervalDurationKey : @1.0,
      AVVideoExpectedSourceFrameRateKey : @30,
      AVVideoAllowFrameReorderingKey : @NO,
    };
    NSDictionary *settings = @{
      AVVideoCodecKey : AVVideoCodecTypeH264,
      AVVideoWidthKey : @(width),
      AVVideoHeightKey : @(height),
      AVVideoCompressionPropertiesKey : compression,
      AVVideoEncoderSpecificationKey : encoder,
    };
    AVAssetWriterInput *input =
        [AVAssetWriterInput assetWriterInputWithMediaType:AVMediaTypeVideo
                                           outputSettings:settings];
    input.expectsMediaDataInRealTime = YES;
    if (![writer canAddInput:input]) {
      copy_error(error, error_capacity, @"writer cannot add video input", 2);
      return NULL;
    }
    [writer addInput:input];
    writer.movieFragmentInterval = CMTimeMake(1, 1);
    writer.shouldOptimizeForNetworkUse = YES;

    AVAssetWriterInputPixelBufferAdaptor *adaptor =
        [AVAssetWriterInputPixelBufferAdaptor
            assetWriterInputPixelBufferAdaptorWithAssetWriterInput:input
                                               sourcePixelBufferAttributes:nil];
    if (![writer startWriting]) {
      copy_error(error, error_capacity, writer.error.localizedDescription, 3);
      return NULL;
    }
    [writer startSessionAtSourceTime:kCMTimeZero];

    EncoreWriterBox *box = [EncoreWriterBox new];
    box.writer = writer;
    box.input = input;
    box.adaptor = adaptor;
    return (__bridge_retained void *)box;
  }
}

int encore_writer_append(void *handle, CVPixelBufferRef pixel_buffer,
                         int64_t pts_microseconds, char *error,
                         size_t error_capacity) {
  @autoreleasepool {
    EncoreWriterBox *box = (__bridge EncoreWriterBox *)handle;
    if (!box.input.readyForMoreMediaData) {
      return copy_error(error, error_capacity, @"encoder backpressure", 1);
    }
    CMTime time = CMTimeMake(pts_microseconds, 1000000);
    if (![box.adaptor appendPixelBuffer:pixel_buffer withPresentationTime:time]) {
      return copy_error(error, error_capacity,
                        box.writer.error.localizedDescription ?: @"frame append failed", 2);
    }
    return 0;
  }
}

int encore_writer_finish(void *handle, char *error, size_t error_capacity) {
  @autoreleasepool {
    EncoreWriterBox *box = (__bridge EncoreWriterBox *)handle;
    [box.input markAsFinished];
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    BOOL finished = [box.writer finishWriting];
#pragma clang diagnostic pop
    if (!finished || box.writer.status != AVAssetWriterStatusCompleted) {
      return copy_error(error, error_capacity,
                        box.writer.error.localizedDescription ?: @"writer finalize failed", 1);
    }
    return 0;
  }
}

void encore_writer_destroy(void *handle) {
  if (handle == NULL) {
    return;
  }
  @autoreleasepool {
    EncoreWriterBox *box = CFBridgingRelease(handle);
    if (box.writer.status == AVAssetWriterStatusWriting) {
      [box.writer cancelWriting];
    }
  }
}
